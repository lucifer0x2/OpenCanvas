//! Private staging and atomic activation. No public API accepts filesystem paths.
use crate::{bundle::*, *};

impl Store {
    /// New identity, authored IDs preserved, revision 0 and empty replay history.
    pub fn import_bundle(&self, slug: &str, bytes: &[u8]) -> Result<Arc<Project>> {
        self.activate_bundle(slug, bytes, false)
    }

    /// Recover the source identity and revision R into an unused destination.
    /// Existing project identities/slugs are never overwritten.
    pub fn restore_bundle(&self, slug: &str, bytes: &[u8]) -> Result<Arc<Project>> {
        self.activate_bundle(slug, bytes, true)
    }

    fn activate_bundle(&self, slug: &str, bytes: &[u8], restore: bool) -> Result<Arc<Project>> {
        self.root.check_path()?;
        validate_slug(slug)?;
        let (manifest, files) = decode(bytes)?;
        let staging = tempfile::Builder::new()
            .prefix(".bundle-")
            .tempdir_in(&self.root.path)?;
        let db = staging.path().join("db.sql");
        write_file(
            &db,
            files
                .get("db.sql")
                .ok_or_else(|| invalid("Database is missing"))?,
        )?;
        let snapshot = bundle_validation::database(&db, &manifest)
            .map_err(|_| invalid("Invalid project database"))?;
        bundle_validation::resources(&snapshot, &files)
            .map_err(|_| invalid("Invalid project resources"))?;
        let assets = staging.path().join("assets");
        fs::create_dir(&assets)?;
        // Only allowlisted names are used, never tar::unpack or client paths.
        for (key, data) in &files {
            if let Some(hash) = key.strip_prefix("assets/") {
                write_file(&assets.join(hash), data)?;
            }
        }
        let project_id = if restore {
            snapshot.project_id.clone()
        } else {
            format!("p_{}", uuid::Uuid::new_v4().simple())
        };
        {
            let mut conn = Connection::open(&db)?;
            configure(&conn, false)?;
            conn.pragma_update(None, "journal_mode", "DELETE")?;
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute("UPDATE project_meta SET project_id=?1,slug=?2,current_revision=?3 WHERE singleton=1", params![project_id, slug, if restore { snapshot.revision as i64 } else { 0 }])?;
            if !restore {
                tx.execute_batch("DELETE FROM commits; DELETE FROM idempotency; UPDATE retention_metadata SET oldest_commit_revision=1 WHERE singleton=1;")?;
            }
            tx.commit()?;
            let actual = read_snapshot(&mut conn)?;
            if actual.project_id != project_id
                || actual.revision != if restore { snapshot.revision } else { 0 }
            {
                return Err(invalid("Staged identity verification failed"));
            }
        }
        File::open(&db)?.sync_all()?;
        sync_dir(&assets)?;
        sync_dir(staging.path())?;
        let registry = self.root.projects.lock().map_err(|_| poisoned())?;
        self.root.check_path()?;
        let destination = self.root.path.join(slug);
        no_symlink(&destination)?;
        if destination.exists() {
            return Err(StorageError::new(
                "PROJECT_EXISTS",
                "A project with this slug already exists",
            ));
        }
        if restore
            && self
                .list_projects()?
                .iter()
                .any(|p| p.project_id == project_id)
        {
            return Err(StorageError::new(
                "PROJECT_EXISTS",
                "The source project identity already exists in this data root",
            ));
        }
        fs::rename(staging.path(), &destination)?;
        sync_dir(&self.root.path)?;
        drop(registry);
        self.open_project(slug)
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
