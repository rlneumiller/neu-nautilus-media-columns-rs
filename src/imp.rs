//! Manual GObject implementation of NautilusMediaColumns.
//!
//! We do NOT use glib-rs's `#[glib::object_subclass]` / `glib::wrapper!` here.
//! Both the modern `glib` crate and the 0.15-line `glib` crate's subclass
//! module only support *static* type registration (`g_type_register_static`),
//! which registers a type permanently against the global GLib type system.
//!
//! Nautilus loads extensions through `GTypeModule`, which requires
//! `g_type_module_register_type` instead -- a type registration that is
//! scoped to the module and can be torn down if the module is unloaded.
//! There is no high-level glib-rs API for this, so we build the raw
//! `GTypeInfo` / `GInterfaceInfo` C structs by hand.

use std::ptr;

use glib_sys::{gpointer, GList, GType};
use gobject_sys::{
    g_type_module_add_interface, g_type_module_register_type, GInterfaceInfo, GObject,
    GObjectClass, GTypeFlags, GTypeInfo, GTypeInstance, GTypeModule,
};
use nautilus4_extension_sys::{
    nautilus_column_new, nautilus_column_provider_get_type, NautilusColumnProviderIface,
};

/// Plain Rust data for one instance. We have no per-instance state today
/// (get_columns is static metadata), but the struct exists so adding
/// instance state later doesn't require touching the registration plumbing.
#[repr(C)]
pub struct NautilusMediaColumnsInstance {
    parent: GObject,
    // No extra fields needed yet.
}

#[repr(C)]
pub struct NautilusMediaColumnsClass {
    parent_class: GObjectClass,
}

/// One safe `Column` description. Mirrors the fields that
/// `nautilus_column_new` expects, but keeps them as owned Rust `String`s
/// until the moment we need to hand C-strings to the FFI call.
pub struct Column {
    pub name: &'static str,
    pub attribute: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

impl Column {
    pub const fn new(
        name: &'static str,
        attribute: &'static str,
        label: &'static str,
        description: &'static str,
    ) -> Self {
        Column {
            name,
            attribute,
            label,
            description,
        }
    }
}

/// Your actual column logic. Kept separate from the FFI plumbing so it's
/// easy to read/extend without touching unsafe code.
fn get_columns() -> Vec<Column> {
    vec![Column::new(
        "media-duration",
        "metadata::duration",
        "Duration",
        "Media duration",
    )]
}

/// C-ABI callback that backs `NautilusColumnProviderIface::get_columns`.
///
/// Signature is fixed by `NautilusColumnProviderIface` in
/// nautilus-extension-sys: `extern fn(gpointer) -> *mut GList`.
/// `instance` is the `NautilusMediaColumns*` GObject pointer Nautilus holds;
/// we don't need to touch it since our columns are static, but real
/// per-instance providers would cast it back with `&*(instance as *const
/// NautilusMediaColumnsInstance)`.
unsafe extern "C" fn column_provider_get_columns(_instance: gpointer) -> *mut GList {
    let columns = get_columns();
    let mut list: *mut GList = ptr::null_mut();

    for col in columns {
        // CString::new will panic on embedded NUL bytes; our literals are
        // static and known-safe, so unwrap is fine here.
        let name = std::ffi::CString::new(col.name).unwrap();
        let attribute = std::ffi::CString::new(col.attribute).unwrap();
        let label = std::ffi::CString::new(col.label).unwrap();
        let description = std::ffi::CString::new(col.description).unwrap();

        // SAFETY: nautilus_column_new is a documented libnautilus-extension
        // entry point. It takes ownership of nothing from us -- it copies
        // the strings internally (standard Nautilus convention) -- and
        // returns a new, owned NautilusColumn* that we hand off to Nautilus
        // via the GList. We do not free it; Nautilus takes ownership of
        // both the GList and its contents once we return them here.
        let column = unsafe {
            nautilus_column_new(
                name.as_ptr(),
                attribute.as_ptr(),
                label.as_ptr(),
                description.as_ptr(),
            )
        };

        // g_list_append style prepend-then-reverse is more efficient than
        // repeated appends, but for a handful of columns clarity wins.
        list = unsafe { glib_sys::g_list_append(list, column as gpointer) };
    }

    list
}

// ---------------------------------------------------------------------
// GObject class/instance init callbacks
// ---------------------------------------------------------------------

// NOTE: GClassInitFunc / GInstanceInitFunc / GInterfaceInitFunc are all
// `Option<unsafe extern "C" fn(...)>` in gobject-sys 0.15 -- plain
// `extern "C" fn` does NOT coerce to that, the `unsafe` is load-bearing
// in the type, not just documentation. Verified against gobject-sys
// 0.15.10's typedefs directly.
unsafe extern "C" fn class_init(_class: gpointer, _class_data: gpointer) {
    // No virtual methods or properties to install on the GObjectClass
    // itself -- everything we expose lives on the ColumnProvider interface,
    // wired up separately via g_type_module_add_interface.
}

unsafe extern "C" fn instance_init(_instance: *mut GTypeInstance, _class: gpointer) {
    // Nothing to initialize: NautilusMediaColumnsInstance has no fields
    // beyond the GObject parent, which GLib already zero-initializes.
}

unsafe extern "C" fn column_provider_interface_init(iface: gpointer, _iface_data: gpointer) {
    let iface = iface as *mut NautilusColumnProviderIface;
    unsafe {
        (*iface).get_columns = Some(column_provider_get_columns);
    }
}

/// Registers `NautilusMediaColumns` against the given `GTypeModule` and
/// attaches the `NautilusColumnProvider` interface to it.
///
/// Must be called from `nautilus_module_initialize`, with the real
/// `GTypeModule*` Nautilus hands us -- this is the whole reason we can't
/// use glib-rs's static `register_type::<T>()`.
pub fn register_type(module: *mut GTypeModule) -> GType {
    let type_info = GTypeInfo {
        class_size: std::mem::size_of::<NautilusMediaColumnsClass>() as u16,
        base_init: None,
        base_finalize: None,
        class_init: Some(class_init),
        class_finalize: None,
        class_data: ptr::null(),
        instance_size: std::mem::size_of::<NautilusMediaColumnsInstance>() as u16,
        n_preallocs: 0,
        instance_init: Some(instance_init),
        value_table: ptr::null(),
    };

    let type_name = std::ffi::CString::new("NautilusMediaColumns").unwrap();

    // SAFETY: `module` is a valid GTypeModule* for the lifetime of this
    // call (Nautilus guarantees this inside nautilus_module_initialize).
    // `type_info` is a valid, fully-initialized GTypeInfo on our stack for
    // the duration of the call, which is all g_type_module_register_type
    // requires -- GLib copies what it needs out of it.
    let media_columns_type = unsafe {
        g_type_module_register_type(
            module,
            gobject_sys::g_object_get_type(),
            type_name.as_ptr(),
            &type_info,
            0 as GTypeFlags, // GTypeFlags is a plain c_uint alias in gobject-sys
                              // 0.15, not a bitflags type -- 0 means "no flags"
                              // (no G_TYPE_FLAG_ABSTRACT / _FINAL / etc.)
        )
    };

    let column_provider_iface_info = GInterfaceInfo {
        interface_init: Some(column_provider_interface_init),
        interface_finalize: None,
        interface_data: ptr::null_mut(),
    };

    // SAFETY: media_columns_type was just registered above and is valid;
    // nautilus_column_provider_get_type() is a stable libnautilus-extension
    // entry point returning the ColumnProvider GInterface's GType.
    unsafe {
        g_type_module_add_interface(
            module,
            media_columns_type,
            nautilus_column_provider_get_type(),
            &column_provider_iface_info,
        );
    }

    media_columns_type
}
