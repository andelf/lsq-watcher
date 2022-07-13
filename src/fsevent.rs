//! fsevent watcher for macOS

use std::ffi::CStr;

use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef},
    string::{kCFStringEncodingUTF8, CFStringGetCStringPtr, CFStringGetLength, CFStringRef},
};
use fsevent_sys as fs;

// The FSEventStreamSetExclusionPaths API has a limit of 8 items.
// If that limit is exceeded, it will fail.
// https://developer.apple.com/documentation/coreservices/1444666-fseventstreamsetexclusionpaths?language=objc
pub const MAX_EXCLUSIONS: usize = 8;

pub const IGNORED_FLAGS: fs::FSEventStreamEventFlags = fs::kFSEventStreamEventFlagItemIsHardlink
    | fs::kFSEventStreamEventFlagItemIsLastHardlink
    | fs::kFSEventStreamEventFlagItemIsSymlink
    | fs::kFSEventStreamEventFlagItemIsDir
    | fs::kFSEventStreamEventFlagItemIsFile;

fn cfstring_array_to_vec(array: CFArrayRef) -> Vec<String> {
    unsafe {
        let mut vec = Vec::new();
        for i in 0..CFArrayGetCount(array) {
            let cfstr = CFArrayGetValueAtIndex(array, i);

            // NOTE: CFStringGetLength is in UTF-16 code pairs
            // let len = CFStringGetLength(cfstr as _);
            let cstr = CFStringGetCStringPtr(cfstr as *const _, kCFStringEncodingUTF8);
            vec.push(CStr::from_ptr(cstr).to_str().unwrap().to_string());
        }
        vec
    }
}

fn cfstring_to_string(cfstr: CFStringRef) -> String {
    unsafe {
        let cstr = CFStringGetCStringPtr(cfstr as *const _, kCFStringEncodingUTF8);
        CStr::from_ptr(cstr).to_str().unwrap().to_string()
    }
}

pub struct FSEvent {}
