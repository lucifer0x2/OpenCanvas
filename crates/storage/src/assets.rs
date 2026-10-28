use crate::*;
impl Project {
    pub(crate) fn asset_path(&self, key: &str) -> Result<PathBuf> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(StorageError::new(
                "PATH_REJECTED",
                "Invalid asset storage key",
            ));
        }
        let assets = guard(&self.path.join("assets"), &self.path)?;
        let path = guard(&assets.join(key), &assets)?;
        if !path.starts_with(&self.path) || !path.is_file() {
            return Err(StorageError::new("PATH_REJECTED", "Invalid asset location"));
        }
        Ok(path)
    }
    pub fn read_asset(&self, id: &str) -> Result<(Asset, Vec<u8>)> {
        self.check_paths()?;
        let conn = self.reader()?;
        let data: Option<String> = conn
            .query_row("SELECT data_json FROM assets WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let asset: Asset = serde_json::from_str(
            &data.ok_or_else(|| StorageError::new("ASSET_NOT_FOUND", "Asset does not exist"))?,
        )?;
        let path = self
            .asset_path(&asset.relative_key)
            .map_err(|_| StorageError::new("ASSET_MISSING", "Asset binary is unavailable"))?;
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_ASSET_BYTES as u64 {
            return Err(StorageError::new(
                "ASSET_INVALID",
                "Asset exceeds the supported size",
            ));
        }
        let bytes = fs::read(path)?;
        if hash(&bytes) != asset.content_hash || bytes.len() as u64 != asset.byte_size {
            return Err(StorageError::new(
                "ASSET_INVALID",
                "Asset content does not match its manifest hash",
            ));
        }
        Ok((asset, bytes))
    }
    pub fn import_asset(&self, request: ImportAssetRequest) -> Result<ImportAssetResult> {
        self.check_paths()?;
        self.request_ids(&request.project_id, &request.idempotency_key)?;
        if request.bytes.is_empty() || request.bytes.len() > MAX_ASSET_BYTES {
            return Err(StorageError::validation(
                "bytes",
                "Asset must contain 1 byte to 10 MiB",
            ));
        }
        let (width, height) = asset_dimensions(&request.media_type, &request.bytes)?;
        let content_hash = hash(&request.bytes);
        let asset_id = format!("asset_{}", &content_hash[..32]);
        let canonical = canonical_json(
            &json!({"projectId":request.project_id,"expectedRevision":request.expected_revision,"mediaType":request.media_type,"hash":content_hash,"font":request.font}),
        );
        let request_hash = hash(&serde_json::to_vec(&canonical)?);
        let mut conn = self.writer.lock().map_err(|_| poisoned())?;
        if let Some(mut result) = replay(&conn, &request.idempotency_key, &request_hash)? {
            let data: String = conn.query_row(
                "SELECT data_json FROM assets WHERE id=?1",
                [&asset_id],
                |r| r.get(0),
            )?;
            result.replayed = true;
            return Ok(ImportAssetResult {
                asset: serde_json::from_str(&data)?,
                result,
            });
        }
        let before = read_snapshot(&mut conn)?;
        if before.revision != request.expected_revision {
            return Err(StorageError::conflict(before.revision));
        }
        let mut candidate = Candidate::from_snapshot(&before);
        let asset = Asset {
            id: asset_id.clone(),
            content_hash: content_hash.clone(),
            media_type: request.media_type.clone(),
            byte_size: request.bytes.len() as u64,
            relative_key: content_hash.clone(),
            width,
            height,
            status: "ready".into(),
        };
        candidate.assets.insert(asset_id.clone(), asset.clone());
        let mut changed = vec![asset_id.clone()];
        if let Some(font) = &request.font {
            if !request.media_type.starts_with("font/")
                || !validate::valid_id(&font.id)
                || font.id == "font_bundled"
                || font.family.is_empty()
                || font.family.len() > 128
                || !(100..=900).contains(&font.weight)
                || !["normal", "italic"].contains(&font.style.as_str())
            {
                return Err(StorageError::validation(
                    "font",
                    "Invalid font registration",
                ));
            }
            candidate.fonts.insert(
                font.id.clone(),
                Font {
                    id: font.id.clone(),
                    family: font.family.clone(),
                    weight: font.weight,
                    style: font.style.clone(),
                    asset_id: Some(asset_id.clone()),
                    bundled_id: None,
                    content_hash: Some(content_hash.clone()),
                    license: font.license.clone(),
                    status: "ready".into(),
                },
            );
            changed.push(font.id.clone());
        }
        candidate.validate()?;
        // Immutable content is durable before the DB can refer to it. A crash here
        // leaves only an orphan, which is deliberately not garbage collected.
        let assets = guard(&self.path.join("assets"), &self.path)?;
        let dest = assets.join(&content_hash);
        no_symlink(&dest)?;
        if dest.exists() {
            if hash(&fs::read(self.asset_path(&content_hash)?)?) != content_hash {
                return Err(StorageError::new(
                    "ASSET_INVALID",
                    "Existing asset content has a different hash",
                ));
            }
        } else {
            let temp = assets.join(format!(".import-{}", uuid::Uuid::new_v4().simple()));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&request.bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &dest)?;
            sync_dir(&assets)?;
        }
        failpoint("after_asset_publish");
        let result = BatchResult {
            project_id: self.project_id.clone(),
            committed_revision: before.revision + 1,
            changed_ids: changed,
            render_status: "pending".into(),
            replayed: false,
        };
        self.persist(
            &mut conn,
            &before,
            &candidate,
            &request.idempotency_key,
            &request_hash,
            &json!([{"op":"import_asset","asset":asset,"font":request.font}]),
            &result,
            &mut BatchMetrics::default(),
        )?;
        Ok(ImportAssetResult { asset, result })
    }
    /// Content-addressed binaries without committed DB refs. Read-only inventory;
    /// no GC is run because render/export handles may still own a binary.
    pub fn orphan_asset_keys(&self) -> Result<Vec<String>> {
        let snapshot = self.snapshot()?;
        let mut referenced: BTreeSet<_> = snapshot
            .assets
            .into_iter()
            .map(|a| a.relative_key)
            .collect();
        // Bundle restore also materializes the pinned fallback font. Its
        // committed bundled-font reference owns that copy without an asset row.
        referenced.extend(snapshot.fonts.into_iter().filter_map(|font| {
            (font.bundled_id.as_deref() == Some("font_bundled"))
                .then_some(font.content_hash)
                .flatten()
        }));
        let mut orphan = Vec::new();
        for entry in fs::read_dir(guard(&self.path.join("assets"), &self.path)?)? {
            let entry = entry?;
            let key = entry.file_name().to_string_lossy().to_string();
            if key.len() == 64
                && key.bytes().all(|b| b.is_ascii_hexdigit())
                && !referenced.contains(&key)
            {
                self.asset_path(&key)?;
                orphan.push(key);
            }
        }
        orphan.sort();
        Ok(orphan)
    }
}
pub(crate) fn asset_dimensions(mime: &str, bytes: &[u8]) -> Result<(Option<u32>, Option<u32>)> {
    match mime {
        "image/png" if bytes.len()>=33 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") && &bytes[12..16]==b"IHDR" => {
            let width=u32::from_be_bytes(bytes[16..20].try_into().unwrap()); let height=u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            if width==0 || height==0 || width as u64*height as u64>40_000_000 { return Err(StorageError::validation("bytes","Image dimensions exceed the supported pixel budget")); } Ok((Some(width),Some(height)))
        },
        "image/jpeg" if bytes.starts_with(b"\xff\xd8\xff") => Ok((None,None)),
        "image/webp" if bytes.len()>=12 && bytes.starts_with(b"RIFF") && &bytes[8..12]==b"WEBP" => Ok((None,None)),
        "font/ttf" if bytes.len()>12 && (bytes.starts_with(&[0,1,0,0]) || bytes.starts_with(b"true")) => Ok((None,None)),
        "font/otf" if bytes.len()>12 && bytes.starts_with(b"OTTO") => Ok((None,None)),
        "font/woff2" if bytes.len()>48 && bytes.starts_with(b"wOF2") => Ok((None,None)),
        _=>Err(StorageError::validation("mediaType","Supported asset types are PNG, JPEG, WebP, TTF, OTF and WOFF2 with matching signatures")),
    }
}
