//! A small, safe layer over the CoreMediaIO C API (`objc2-core-media-io`),
//! which cidre does not bind: list cameras, ask whether one is running
//! somewhere, and listen for changes.
//!
//! The only `unsafe` in detection lives here. Each call passes pointers to
//! locals that outlive the call, with sizes taken from the same types, and
//! the listener block stays alive for as long as it is registered.
//! Ported from Muesli's `CameraActivityMonitor.swift` (MIT), which uses the
//! DAL API rather than `AVCaptureDevice` discovery, whose own main-queue HAL
//! listeners stalled on AirPods.

#![allow(
    unsafe_code,
    reason = "FFI to CoreMediaIO; every call is wrapped safely here"
)]

use std::ffi::c_void;
use std::ptr;

use block2::RcBlock;
use objc2_core_media_io::{
    CMIOObjectAddPropertyListenerBlock, CMIOObjectGetPropertyData, CMIOObjectGetPropertyDataSize,
    CMIOObjectID, CMIOObjectPropertyAddress, CMIOObjectPropertySelector,
    CMIOObjectRemovePropertyListenerBlock, kCMIODevicePropertyDeviceIsRunningSomewhere,
    kCMIODevicePropertyScopeInput, kCMIODevicePropertyStreams, kCMIOHardwarePropertyDevices,
    kCMIOObjectPropertyElementMain, kCMIOObjectPropertyElementWildcard,
    kCMIOObjectPropertyScopeGlobal, kCMIOObjectPropertyScopeWildcard, kCMIOObjectSystemObject,
};

use crate::ports::Waker;

/// The CoreMediaIO system object, which lists the devices.
pub(super) const SYSTEM: CMIOObjectID = kCMIOObjectSystemObject;

/// `kCMIOHardwareNoError`.
const NO_ERROR: i32 = 0;

/// The device list on the system object.
pub(super) const DEVICES: CMIOObjectPropertyAddress = address(
    kCMIOHardwarePropertyDevices,
    kCMIOObjectPropertyScopeGlobal,
    kCMIOObjectPropertyElementMain,
);

/// "Running somewhere", in any scope and element (Muesli).
pub(super) const RUNNING_SOMEWHERE: CMIOObjectPropertyAddress = address(
    kCMIODevicePropertyDeviceIsRunningSomewhere,
    kCMIOObjectPropertyScopeWildcard,
    kCMIOObjectPropertyElementWildcard,
);

/// A device's input streams: a camera has at least one.
const INPUT_STREAMS: CMIOObjectPropertyAddress = address(
    kCMIODevicePropertyStreams,
    kCMIODevicePropertyScopeInput,
    kCMIOObjectPropertyElementMain,
);

const fn address(
    selector: CMIOObjectPropertySelector,
    scope: u32,
    element: u32,
) -> CMIOObjectPropertyAddress {
    CMIOObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

/// Every device with input streams (the cameras), or the failing status.
pub(super) fn cameras() -> Result<Vec<CMIOObjectID>, i32> {
    let size = data_size(SYSTEM, &DEVICES)?;
    let mut devices = vec![0 as CMIOObjectID; size as usize / size_of::<CMIOObjectID>()];
    let used = read(
        SYSTEM,
        &DEVICES,
        devices.as_mut_ptr().cast(),
        size_of_val(devices.as_slice()),
    )?;
    devices.truncate(used as usize / size_of::<CMIOObjectID>());
    devices.retain(|device| data_size(*device, &INPUT_STREAMS).is_ok_and(|size| size > 0));
    Ok(devices)
}

/// Whether `device` is running for any process.
pub(super) fn is_running_somewhere(device: CMIOObjectID) -> bool {
    let mut running: u32 = 0;
    let read = read(
        device,
        &RUNNING_SOMEWHERE,
        ptr::from_mut(&mut running).cast(),
        size_of::<u32>(),
    );
    read.is_ok() && running != 0
}

fn data_size(object: CMIOObjectID, address: &CMIOObjectPropertyAddress) -> Result<u32, i32> {
    let mut size: u32 = 0;
    // SAFETY: `address` and `size` are live locals for the whole call; no qualifier.
    let status = unsafe {
        CMIOObjectGetPropertyDataSize(
            object,
            ptr::from_ref(address),
            0,
            ptr::null(),
            &raw mut size,
        )
    };
    if status == NO_ERROR {
        Ok(size)
    } else {
        Err(status)
    }
}

/// Reads a property into `data` (`capacity` bytes); returns the bytes used.
fn read(
    object: CMIOObjectID,
    address: &CMIOObjectPropertyAddress,
    data: *mut c_void,
    capacity: usize,
) -> Result<u32, i32> {
    let capacity = u32::try_from(capacity).map_err(|_| -1)?;
    let mut used: u32 = 0;
    // SAFETY: `data` points at `capacity` writable bytes owned by the caller
    // for the whole call; `address` and `used` are live locals.
    let status = unsafe {
        CMIOObjectGetPropertyData(
            object,
            ptr::from_ref(address),
            0,
            ptr::null(),
            capacity,
            &raw mut used,
            data,
        )
    };
    if status == NO_ERROR {
        Ok(used)
    } else {
        Err(status)
    }
}

type ListenerBlock = RcBlock<dyn Fn(u32, *mut CMIOObjectPropertyAddress)>;

/// A registered CoreMediaIO listener that wakes the detector. Dropping it
/// removes it.
pub(super) struct CmioListener {
    object: CMIOObjectID,
    address: CMIOObjectPropertyAddress,
    block: ListenerBlock,
}

impl CmioListener {
    /// Listens to `address` on `object`. With no dispatch queue, CoreMediaIO
    /// calls the block directly; it only wakes the detector.
    pub(super) fn add(
        object: CMIOObjectID,
        address: CMIOObjectPropertyAddress,
        waker: &Waker,
    ) -> Result<Self, i32> {
        let waker = waker.clone();
        let block: ListenerBlock = RcBlock::new(
            move |_count: u32, _addresses: *mut CMIOObjectPropertyAddress| waker.wake(),
        );
        // SAFETY: CoreMediaIO copies the block (Block_copy) and keeps it until
        // the matching remove in `Drop`; `address` is a live local.
        let status = unsafe {
            CMIOObjectAddPropertyListenerBlock(
                object,
                &raw const address,
                None,
                RcBlock::as_ptr(&block),
            )
        };
        if status == NO_ERROR {
            Ok(Self {
                object,
                address,
                block,
            })
        } else {
            Err(status)
        }
    }
}

impl Drop for CmioListener {
    fn drop(&mut self) {
        let block = RcBlock::as_ptr(&self.block);
        // SAFETY: the same object, address and block that were registered.
        let status = unsafe {
            CMIOObjectRemovePropertyListenerBlock(self.object, &raw const self.address, None, block)
        };
        if status != NO_ERROR {
            log::debug!(target: "detect", "could not remove a camera listener (OS status {status})");
        }
    }
}
