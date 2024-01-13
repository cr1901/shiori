use std::fs;
use std::ffi::c_void;
use std::io::Write;
use std::mem::{self, MaybeUninit};
use std::error;
use std::time::Duration;

// use windows_sys::{
//     core::*, Win32::Foundation::*, Win32::Storage::FileSystem::*, Win32::System::Ioctl::*,
//     Win32::System::IO::*,
// };

use windows::Win32::System::Ioctl::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::*;
use windows::core::HSTRING;

fn main() -> Result<(), Box<dyn error::Error>> {
    unsafe {
        let handle = CreateFileW(
            &HSTRING::from("\\\\.\\A:"),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?;

        let mut dg = MaybeUninit::<DISK_GEOMETRY>::uninit();
        let mut byte_ret: u32 = 0;
        DeviceIoControl(
            handle,                        // device to be queried
            IOCTL_DISK_GET_DRIVE_GEOMETRY, // operation to perform
            None,
            0, // no input buffer
            Some(dg.as_mut_ptr() as *mut c_void),
            mem::size_of::<DISK_GEOMETRY>() as u32, // output buffer
            Some(&mut byte_ret),                          // # bytes returned
            None,
        )?; // synchronous I/O

        let dg = dg.assume_init();
        println!("{}, {}, {:?}, {}, {}", dg.BytesPerSector, dg.Cylinders, dg.MediaType, dg.SectorsPerTrack, dg.TracksPerCylinder);


        DeviceIoControl(
            handle,                        // device to be queried
            FSCTL_LOCK_VOLUME, // operation to perform
            None,
            0, // no input buffer
            None,
            0, // output buffer
            Some(&mut byte_ret),                          // # bytes returned
            None,
        )?;

        println!("Volume locked!");

        DeviceIoControl(
            handle,                        // device to be queried
            FSCTL_DISMOUNT_VOLUME, // operation to perform
            None,
            0, // no input buffer
            None,
            0, // output buffer
            Some(&mut byte_ret),                          // # bytes returned
            None,
        )?;

        println!("Volume dismounted!");

        let mut buf = Box::<[u8; 8*1024]>::new([0; 8*1024]);

        if dg.MediaType.0 == 2 {
            std::thread::sleep(Duration::from_secs(2));
        }

        // SetFilePointer(handle, 512, None, FILE_BEGIN);

        let mut outp = fs::File::create("foo.ima")?;
        let mut i = 0;
        let mut byte_incr;

        loop {
            let res = ReadFile(handle,
                Some(&mut *buf),
                Some(&mut byte_ret),
                None);

            if res.is_err() {
                println!("Read error {}", res.unwrap_err());
                let next_possible_sector = if byte_ret == 0 {
                    i + (8192 as u32/512)
                } else {
                    i + (byte_ret as u32/512)
                };

                println!("Moving to {}", next_possible_sector * 512);
                SetFilePointer(handle, (next_possible_sector * 512) as i32, None, FILE_BEGIN);
                buf[byte_ret as usize..].fill(0);

                if byte_ret == 0 {
                    if next_possible_sector as u64 > (dg.Cylinders as u64 * dg.SectorsPerTrack as u64 * dg.TracksPerCylinder as u64) {
                        break;
                    }
                    byte_incr = 8192;
                } else {
                    byte_incr = next_possible_sector * 512;
                }
            } else {
                byte_incr = byte_ret;
                if byte_ret == 0 {
                    break;
                }
            }

            outp.write(&*buf)?;
            //println!("Sector {} Data {:X?}", i, buf);
            if byte_ret > 0 {
                println!("Sector {}-{}, {}", i, i + (byte_ret/512) - 1, byte_ret);
            }

            i += byte_incr/512;
        };

        DeviceIoControl(
            handle,                        // device to be queried
            FSCTL_UNLOCK_VOLUME, // operation to perform
            None,
            0, // no input buffer
            None,
            0, // output buffer
            Some(&mut byte_ret),                          // # bytes returned
            None,
        )?;

        println!("Volume unlocked!");

        CloseHandle(handle)?;

        // TODO: Physical sector size in IOCTL_STORAGE_QUERY_PROPERTY.
    }

    Ok(())
}
