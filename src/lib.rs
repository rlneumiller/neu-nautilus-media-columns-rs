mod imp;

use std::sync::OnceLock;

use glib_sys::GType;
use gobject_sys::GTypeModule;

/// The GType we get back from `imp::register_type` once Nautilus has
/// initialized us. `nautilus_module_list_types` needs to hand this back
/// out later, after `nautilus_module_initialize` has already returned --
/// so it has to live somewhere outside either function's stack frame.
///
/// `OnceLock` rather than `lazy_static`/`Mutex`: this is written exactly
/// once, during `nautilus_module_initialize`, and only read afterwards.
/// Nautilus's module-loading contract guarantees initialize() completes
/// (single-threaded, at load time) before list_types() is ever called, so
/// there's no real concurrent-write hazard -- OnceLock just encodes that
/// guarantee in the type instead of asserting it by convention.
static MEDIA_COLUMNS_TYPE: OnceLock<GType> = OnceLock::new();

/// Called once by Nautilus when it loads this .so via GTypeModule.
/// This is where (and only where) GLib type registration against `module`
/// is valid -- see imp::register_type for why glib-rs's static
/// registration helpers don't work for GTypeModule-loaded plugins.
#[unsafe(no_mangle)]
pub extern "C" fn nautilus_module_initialize(module: *mut GTypeModule) {
    let gtype = imp::register_type(module);
    // .set() rather than overwrite-on-reload: if Nautilus somehow called
    // initialize twice without an intervening shutdown, we want a loud
    // logic-error signal rather than silently rebinding to a new GType
    // while old instances (if any) still reference the first one.
    if MEDIA_COLUMNS_TYPE.set(gtype).is_err() {
        eprintln!(
            "nautilus-media-columns-rs: nautilus_module_initialize called \
             more than once without an intervening shutdown"
        );
    }
}

/// Called once by Nautilus before unloading the module.
/// We hold no heap allocations, open handles, or background threads that
/// need explicit teardown, so this is a no-op -- but it must still exist
/// with C linkage, since Nautilus calls it unconditionally.
#[unsafe(no_mangle)]
pub extern "C" fn nautilus_module_shutdown() {}

/// Called by Nautilus after initialize() to ask which GTypes this module
/// provides. We hand back the single GType we registered, wrapped in a
/// heap-allocated array Nautilus expects to read (not free -- module
/// lifetime convention here is "valid until module unload", matching the
/// static's lifetime).
///
/// `types` per the upstream header is `(array length=num_types): array of
/// GType *` -- i.e. Nautilus reads `*num_types` contiguous GType values
/// starting at `*types`. With exactly one type, pointing at our single
/// static GType works because a 1-element array IS just that element's
/// address. If you add more provider types later (PropertyPageProvider,
/// MenuProvider, ...) you'll need an actual `[GType; N]` (or a
/// `Vec<GType>` leaked/static-allocated) holding all of them contiguously
/// -- a single OnceLock<GType> per type won't satisfy the "contiguous
/// array" contract on its own.
#[unsafe(no_mangle)]
pub extern "C" fn nautilus_module_list_types(
    types: *mut *const GType,
    num_types: *mut std::os::raw::c_int,
) {
    let gtype = MEDIA_COLUMNS_TYPE
        .get()
        .expect("nautilus_module_list_types called before nautilus_module_initialize");

    // SAFETY: Nautilus calls this only after initialize(), passing valid
    // out-pointers for both `types` and `num_types`. We point `types` at
    // our static's address rather than allocating: the GType value lives
    // for the process lifetime once OnceLock is set, satisfying the
    // "valid until unload" contract without needing a leak or an
    // allocator round-trip.
    unsafe {
        *types = gtype as *const GType;
        *num_types = 1;
    }
}
