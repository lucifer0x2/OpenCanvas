//! One instance of the spike .wasm (../wasm/src/lib.rs) behind its C-ABI: copy bytes in with
//! `alloc`, call `init`/`load_scene`/`render`, copy the RGBA pixmap out.
use std::time::Instant;
use wasmtime::{Engine, Instance, Memory, Module, Result, Store, TypedFunc};

pub fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

pub struct Spike {
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    dealloc: TypedFunc<(u32, u32), ()>,
    render: TypedFunc<f32, i32>,
    instance: Instance,
}

impl Spike {
    /// Instantiates `module` (no imports: the cdylib needs none).
    pub fn new(engine: &Engine, module: &Module) -> Result<Spike> {
        let mut store = Store::new(engine, ());
        let instance = Instance::new(&mut store, module, &[])?;
        let memory = instance.get_memory(&mut store, "memory").expect("memory export");
        let alloc = instance.get_typed_func(&mut store, "alloc")?;
        let dealloc = instance.get_typed_func(&mut store, "dealloc")?;
        let render = instance.get_typed_func(&mut store, "render")?;
        Ok(Spike { store, memory, alloc, dealloc, render, instance })
    }

    fn put(&mut self, bytes: &[u8]) -> Result<(u32, u32)> {
        let len = bytes.len() as u32;
        let ptr = self.alloc.call(&mut self.store, len)?;
        self.memory.write(&mut self.store, ptr as usize, bytes)?;
        Ok((ptr, len))
    }

    fn free(&mut self, (ptr, len): (u32, u32)) -> Result<()> {
        self.dealloc.call(&mut self.store, (ptr, len))
    }

    fn func<P: wasmtime::WasmParams, R: wasmtime::WasmResults>(&mut self, name: &str) -> Result<TypedFunc<P, R>> {
        self.instance.get_typed_func(&mut self.store, name)
    }

    fn check(name: &str, code: i32) -> Result<()> {
        if code != 0 {
            wasmtime::bail!("{name} returned {code}");
        }
        Ok(())
    }

    /// Font parse + image PNG decode inside the module.
    pub fn init(&mut self, font: &[u8], png: &[u8]) -> Result<()> {
        let (f, p) = (self.put(font)?, self.put(png)?);
        let init: TypedFunc<(u32, u32, u32, u32), i32> = self.func("init")?;
        let code = init.call(&mut self.store, (f.0, f.1, p.0, p.1))?;
        self.free(f)?;
        self.free(p)?;
        Self::check("init", code)
    }

    /// Scene JSON copy + parse inside the module.
    pub fn load_scene(&mut self, json: &[u8]) -> Result<()> {
        let s = self.put(json)?;
        let load: TypedFunc<(u32, u32), i32> = self.func("load_scene")?;
        let code = load.call(&mut self.store, s)?;
        self.free(s)?;
        Self::check("load_scene", code)
    }

    /// Paint only; the pixmap stays in linear memory.
    pub fn render(&mut self, scale: f32) -> Result<()> {
        let code = self.render.call(&mut self.store, scale)?;
        Self::check("render", code)
    }

    /// (width, height, premultiplied RGBA) of the last render.
    pub fn pixels(&mut self) -> Result<(u32, u32, Vec<u8>)> {
        let ptr: TypedFunc<(), u32> = self.func("pixels_ptr")?;
        let len: TypedFunc<(), u32> = self.func("pixels_len")?;
        let w: TypedFunc<(), u32> = self.func("width")?;
        let h: TypedFunc<(), u32> = self.func("height")?;
        let (ptr, len) = (ptr.call(&mut self.store, ())?, len.call(&mut self.store, ())?);
        let (w, h) = (w.call(&mut self.store, ())?, h.call(&mut self.store, ())?);
        let mut out = vec![0u8; len as usize];
        self.memory.read(&self.store, ptr as usize, &mut out)?;
        Ok((w, h, out))
    }

    /// Linear memory size in bytes (what the page would hold for this instance).
    pub fn memory_bytes(&self) -> usize {
        self.memory.data_size(&self.store)
    }
}
