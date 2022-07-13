use anyhow::Result;
use cf::base::CFRelease;
use cf::string::kCFStringEncodingUTF8;
use itertools::izip;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::Path;
use std::time::Duration;
use std::{env, ptr, slice, thread};

// only available on unix platforms
use std::os::unix::ffi::OsStrExt;

use fsevent_sys as fs;
// only part of CoreFoundation
// use fsevent_sys::core_foundation as cf;
use core_foundation as cf;

type dispatch_queue_t = *mut c_void;
type dispatch_queue_attr_t = *mut c_void;
type dispatch_object_t = *mut c_void;

extern "C" {

    pub fn dispatch_queue_create(
        label: *const c_char,
        attr: dispatch_queue_attr_t,
    ) -> dispatch_queue_t;

    pub fn dispatch_release(object: dispatch_object_t);

    pub fn FSEventStreamSetDispatchQueue(
        stream: fs::ConstFSEventStreamRef,
        queue: dispatch_queue_t,
    );
}

static mut TEST_CTX: i64 = 0;

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum FSEventStramaEventFlag {
    None = fs::kFSEventStreamEventFlagNone,
    MustScanSubDirs = fs::kFSEventStreamEventFlagMustScanSubDirs,
    UserDropped = fs::kFSEventStreamEventFlagUserDropped,
    KernelDropped = fs::kFSEventStreamEventFlagKernelDropped,
    EventIdsWrapped = fs::kFSEventStreamEventFlagEventIdsWrapped,
    HistoryDone = fs::kFSEventStreamEventFlagHistoryDone,
    RootChanged = fs::kFSEventStreamEventFlagRootChanged,
    Mount = fs::kFSEventStreamEventFlagMount,
    Unmount = fs::kFSEventStreamEventFlagUnmount,
    ItemCreated = fs::kFSEventStreamEventFlagItemCreated,
    ItemRemoved = fs::kFSEventStreamEventFlagItemRemoved,
    ItemInodeMetaMod = fs::kFSEventStreamEventFlagItemInodeMetaMod,
    ItemRenamed = fs::kFSEventStreamEventFlagItemRenamed,
    ItemModified = fs::kFSEventStreamEventFlagItemModified,
    ItemFinderInfoMod = fs::kFSEventStreamEventFlagItemFinderInfoMod,
    ItemChangeOwner = fs::kFSEventStreamEventFlagItemChangeOwner,
    ItemXattrMod = fs::kFSEventStreamEventFlagItemXattrMod,
    ItemIsFile = fs::kFSEventStreamEventFlagItemIsFile,
    ItemIsDir = fs::kFSEventStreamEventFlagItemIsDir,
    ItemIsSymlink = fs::kFSEventStreamEventFlagItemIsSymlink,
    OwnEvent = fs::kFSEventStreamEventFlagOwnEvent,
    ItemIsHardlink = fs::kFSEventStreamEventFlagItemIsHardlink,
    ItemIsLastHardlink = fs::kFSEventStreamEventFlagItemIsLastHardlink,
    ItemCloned = fs::kFSEventStreamEventFlagItemCloned,
}

pub trait EventFlagExt {
    fn as_event_flags(&self) -> Vec<FSEventStramaEventFlag>;
}

impl EventFlagExt for u32 {
    fn as_event_flags(&self) -> Vec<FSEventStramaEventFlag> {
        use FSEventStramaEventFlag::*;
        [
            None,
            MustScanSubDirs,
            UserDropped,
            KernelDropped,
            EventIdsWrapped,
            HistoryDone,
            RootChanged,
            Mount,
            Unmount,
            ItemCreated,
            ItemRemoved,
            ItemInodeMetaMod,
            ItemRenamed,
            ItemModified,
            ItemFinderInfoMod,
            ItemChangeOwner,
            ItemXattrMod,
            ItemIsFile,
            ItemIsDir,
            ItemIsSymlink,
            OwnEvent,
            ItemIsHardlink,
            ItemIsLastHardlink,
            ItemCloned,
        ]
        .into_iter()
        .filter(|&flag| self & flag as u32 != 0)
        .collect()
    }
}

extern "C" fn callback(
    stream: fs::FSEventStreamRef,
    info: *mut libc::c_void,
    num_events: libc::size_t,                        // size_t numEvents
    event_paths: *mut libc::c_void,                  // void *eventPaths
    event_flags: *const fs::FSEventStreamEventFlags, // const FSEventStreamEventFlags eventFlags[]
    event_ids: *const fs::FSEventStreamEventId,      // const FSEventStreamEventId eventIds[]
) {
    println!("callback called!");
    println!("events {}", num_events);

    let epaths = unsafe {
        slice::from_raw_parts(event_paths as *const *const i8, num_events)
            .into_iter()
            .map(|&p| CStr::from_ptr(p))
            .collect::<Vec<_>>()
    };

    let eflags = unsafe { slice::from_raw_parts(event_flags, num_events) };
    let eids = unsafe { slice::from_raw_parts(event_ids, num_events) };

    for (id, path, flags) in izip!(eids, epaths, eflags) {
        println!("{} {:?}", id, path);
        for flag in flags.as_event_flags() {
            println!("  {:?}", flag);
        }
    }

    //    println!("flags => {:?}", eflags);
    //  println!("ids => {:?}", eids);
    //println!("paths => {:?}", epaths);

    /*callback_impl(
        stream_ref,
        info,
        num_events,
        event_paths,
        event_flags,
        event_ids,
    )*/
}

extern "C" fn retain_callback(p: *const c_void) -> *const c_void {
    println!("!! calling retain!");
    return p;
}

extern "C" fn release_callback(p: *const c_void) {
    println!("!! calling release!");
}

fn main() -> Result<()> {
    let top_dir = {
        let arg = env::args().nth(1).unwrap();
        let path = Path::new(&arg);
        path.canonicalize()?
    };
    println!("watching dir => {:?}", top_dir);

    unsafe {
        let label = CString::new("com.logseq.watcher").unwrap();

        let queue = dispatch_queue_create(label.as_ptr(), ptr::null_mut());
        println!("queue => {:?  }", queue);

        let context = fs::FSEventStreamContext {
            version: 0,
            info: &mut TEST_CTX as *mut _ as *mut c_void,
            retain: Some(retain_callback),
            release: Some(release_callback),
            copy_description: None,
        };
        let cstr = CString::new(top_dir.as_os_str().as_bytes()).unwrap();
        let s = cf::string::CFStringCreateWithCString(
            ptr::null_mut(),
            cstr.as_ptr(),
            cf::string::kCFStringEncodingUTF8,
        );
        println!("s => {:?}", s);
        let ds =
            cf::array::CFArrayCreate(ptr::null_mut(), &(s as *const c_void), 1, ptr::null_mut());
        println!("ds => {:?}", ds);

        let stream = fs::FSEventStreamCreate(
            ptr::null_mut(),
            callback,
            &context,
            ds as _,
            fs::kFSEventStreamEventIdSinceNow, // Or a previous event ID
            //88465408,
            0.0,
            // fs::kFSEventStreamCreateFlagNone,
            fs::kFSEventStreamCreateFlagFileEvents,
        );

        println!("stream => {:?}", stream);

        CFRelease(ds as *mut c_void);
        CFRelease(s as *mut c_void);

        FSEventStreamSetDispatchQueue(stream, queue);

        fs::FSEventStreamStart(stream);

        // get desc: CFString to &str
        let mut buf = [0u8; 1024];
        let cfdesc = fs::FSEventStreamCopyDescription(stream);
        cf::string::CFStringGetCString(
            cfdesc as _,
            buf.as_mut_ptr() as _,
            1024,
            kCFStringEncodingUTF8,
        );
        println!(
            "desc => {}",
            CStr::from_ptr(buf.as_ptr() as _).to_str().unwrap()
        );

        thread::sleep(Duration::from_millis(1000_000));

        // end - deinit
        // Stop -> Invalidate -> Release
        fs::FSEventStreamStop(stream);
        fs::FSEventStreamInvalidate(stream);
        fs::FSEventStreamRelease(stream);

        dispatch_release(queue);

        println!("deinit");
        thread::sleep(Duration::from_millis(100_000));
    }
    Ok(())
}
