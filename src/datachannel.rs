use std::ffi::{c_void, CStr};
use std::mem;
use std::os::raw::c_char;
use std::slice;

use datachannel_sys as sys;

pub trait MakeDataChannel<D>
where
    D: DataChannel,
{
    fn make(&self) -> D;
}

impl<F, D> MakeDataChannel<D> for F
where
    F: Fn() -> D,
    D: DataChannel,
{
    fn make(&self) -> D {
        self()
    }
}

#[allow(unused_variables)]
pub trait DataChannel {
    fn on_open(&mut self) {}
    fn on_closed(&mut self) {}
    fn on_error(&mut self, err: &str) {}
    fn on_message(&mut self, msg: &[u8]) {}
}

#[repr(C)]
pub struct RtcDataChannel<D> {
    id: i32,
    dc: D,
}

impl<D> RtcDataChannel<D>
where
    D: DataChannel,
{
    pub(crate) fn new(id: i32, dc: D) -> Self {
        unsafe {
            let rtc_dc = RtcDataChannel { id, dc };
            sys::rtcSetUserPointer(rtc_dc.id, &rtc_dc as *const _ as *mut c_void);

            sys::rtcSetOpenCallback(rtc_dc.id, Some(RtcDataChannel::<D>::open_cb));
            sys::rtcSetClosedCallback(rtc_dc.id, Some(RtcDataChannel::<D>::closed_cb));
            sys::rtcSetErrorCallback(rtc_dc.id, Some(RtcDataChannel::<D>::error_cb));
            sys::rtcSetMessageCallback(rtc_dc.id, Some(RtcDataChannel::<D>::message_cb));

            rtc_dc
        }
    }

    unsafe extern "C" fn open_cb(ptr: *mut c_void) {
        let rtc_dc: &mut RtcDataChannel<D> = mem::transmute(ptr);
        rtc_dc.dc.on_open()
    }

    unsafe extern "C" fn closed_cb(ptr: *mut c_void) {
        let rtc_dc: &mut RtcDataChannel<D> = mem::transmute(ptr);
        rtc_dc.dc.on_closed()
    }

    unsafe extern "C" fn error_cb(err: *const c_char, ptr: *mut c_void) {
        let rtc_dc: &mut RtcDataChannel<D> = mem::transmute(ptr);
        let err = CStr::from_ptr(err).to_str().unwrap();
        rtc_dc.dc.on_error(err)
    }

    unsafe extern "C" fn message_cb(msg: *const c_char, size: i32, ptr: *mut c_void) {
        let rtc_dc: &mut RtcDataChannel<D> = mem::transmute(ptr);
        let msg = if size < 0 {
            CStr::from_ptr(msg).to_bytes()
        } else {
            slice::from_raw_parts(msg as *const u8, size as usize)
        };
        rtc_dc.dc.on_message(msg)
    }

    pub fn label(&self) -> String {
        unsafe {
            let mut buf: Vec<u8> = vec![0; 512];
            sys::rtcGetDataChannelLabel(self.id, buf.as_mut_ptr() as *mut c_char, 512 as i32);
            String::from_utf8(buf).unwrap()
        }
    }

    pub fn send(&mut self, msg: &[u8]) {
        unsafe { sys::rtcSendMessage(self.id, msg.as_ptr() as *const c_char, msg.len() as i32) };
    }
}

impl<D> Drop for RtcDataChannel<D> {
    fn drop(&mut self) {
        unsafe { sys::rtcDeleteDataChannel(self.id) };
    }
}