use crate::*;
pub(crate) struct Root {
    pub(crate) path: PathBuf,
    _lock: File,
    pub(crate) projects: Mutex<BTreeMap<String, Weak<Project>>>,
}
#[derive(Clone)]
pub struct Store {
    root: Arc<Root>,
}
impl Store {
    /// Explicit root selection. Does not create any project or touch a default home.
    pub fn open(data_root: impl AsRef<Path>) -> Result<Self> {
        version_check()?;
        fs::create_dir_all(data_root.as_ref())?;
        let path = fs::canonicalize(data_root.as_ref())?;
        let lock_path = path.join(".instance.lock");
        no_symlink(&lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock_exclusive().map_err(|_| {
            StorageError::new("DATA_ROOT_LOCKED", "Another backend owns this data root")
        })?;
        Ok(Self {
            root: Arc::new(Root {
                path,
                _lock: lock,
                projects: Mutex::new(BTreeMap::new()),
            }),
        })
    }
    pub fn create_project(&self, slug: &str, name: &str) -> Result<Arc<Project>> {
        validate_slug(slug)?;
        if name.is_empty() || name.len() > 256 {
            return Err(StorageError::validation(
                "name",
                "Project name must contain 1–256 bytes",
            ));
        }
        // Root registry lock also serializes create/open for one slug.
        let mut registry = self.root.projects.lock().map_err(|_| poisoned())?;
        let path = self.root.path.join(slug);
        no_symlink(&path)?;
        fs::create_dir(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                StorageError::new("PROJECT_EXISTS", "A project with this slug already exists")
            } else {
                e.into()
            }
        })?;
        let path = guard(&path, &self.root.path)?;
        fs::create_dir(path.join("assets"))?;
        sync_dir(&path)?;
        sync_dir(&self.root.path)?;
        let mut conn = Connection::open(path.join("db.sql"))?;
        configure(&conn, true)?;
        let project_id = format!("p_{}", uuid::Uuid::new_v4().simple());
        initialize(&mut conn, &project_id, slug, name)?;
        sync_dir(&path)?;
        let project = Arc::new(Project {
            root: self.root.clone(),
            path,
            project_id: project_id.clone(),
            writer: Mutex::new(conn),
        });
        registry.insert(slug.into(), Arc::downgrade(&project));
        registry.insert(project_id, Arc::downgrade(&project));
        Ok(project)
    }
    pub fn list_projects(&self) -> Result<Vec<ProjectMeta>> {
        let mut list = Vec::new();
        for entry in fs::read_dir(&self.root.path)? {
            let entry = entry?;
            let slug = entry.file_name().to_string_lossy().to_string();
            if validate_slug(&slug).is_err() || !entry.file_type()?.is_dir() {
                continue;
            }
            let path = guard(&entry.path(), &self.root.path)?;
            let db = path.join("db.sql");
            if !db.exists() {
                continue;
            }
            guard(&db, &path)?;
            no_symlink(&path.join("db.sql-wal"))?;
            no_symlink(&path.join("db.sql-shm"))?;
            let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            configure(&conn, false)?;
            let meta = read_meta(&conn)?;
            if meta.slug != slug {
                return Err(StorageError::new(
                    "PROJECT_INVALID",
                    "Project manifest does not match its directory",
                ));
            }
            list.push(meta);
        }
        list.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(list)
    }
    pub fn open_project(&self, id_or_slug: &str) -> Result<Arc<Project>> {
        if !validate::valid_id(id_or_slug) {
            return Err(StorageError::validation(
                "projectId",
                "Invalid project identifier",
            ));
        }
        let mut registry = self.root.projects.lock().map_err(|_| poisoned())?;
        if let Some(existing) = registry.get(id_or_slug).and_then(Weak::upgrade) {
            existing.check_paths()?;
            return Ok(existing);
        }
        let slug = if validate_slug(id_or_slug).is_ok() && self.root.path.join(id_or_slug).exists()
        {
            id_or_slug.to_string()
        } else {
            self.list_projects()?
                .into_iter()
                .find(|m| m.project_id == id_or_slug)
                .map(|m| m.slug)
                .ok_or_else(|| StorageError::new("PROJECT_NOT_FOUND", "Project does not exist"))?
        };
        let path = guard(&self.root.path.join(&slug), &self.root.path)?;
        guard(&path.join("assets"), &path)?;
        guard(&path.join("db.sql"), &path)?;
        no_symlink(&path.join("db.sql-wal"))?;
        no_symlink(&path.join("db.sql-shm"))?;
        let conn =
            Connection::open_with_flags(path.join("db.sql"), OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        configure(&conn, true)?;
        let meta = read_meta(&conn)?;
        if meta.schema_version != SCHEMA_VERSION {
            return Err(StorageError::new(
                "SCHEMA_UNSUPPORTED",
                "Project schema requires a supported migration before writing",
            ));
        }
        if meta.slug != slug {
            return Err(StorageError::new(
                "PROJECT_INVALID",
                "Project manifest does not match its directory",
            ));
        }
        let project = Arc::new(Project {
            root: self.root.clone(),
            path,
            project_id: meta.project_id.clone(),
            writer: Mutex::new(conn),
        });
        registry.insert(slug, Arc::downgrade(&project));
        registry.insert(meta.project_id, Arc::downgrade(&project));
        Ok(project)
    }
}
