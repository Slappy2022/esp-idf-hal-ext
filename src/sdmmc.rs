use core::ffi::c_void;
use esp_idf_sys::*;

pub struct Sdmmc {
    base_path: &'static str,
    card: *mut sdmmc_card_t,
}

const DEFAULT_HOST_CONFIG: sdmmc_host_t = sdmmc_host_t {
    flags: 0x000F, // 1 bit, 4 bit, 8 bit, ddr
    slot: 1,
    max_freq_khz: 20_000,
    io_voltage: 3.3,
    init: Some(sdmmc_host_init),
    set_bus_width: Some(sdmmc_host_set_bus_width),
    get_bus_width: Some(sdmmc_host_get_slot_width),
    set_bus_ddr_mode: Some(sdmmc_host_set_bus_ddr_mode),
    set_card_clk: Some(sdmmc_host_set_card_clk),
    do_transaction: Some(sdmmc_host_do_transaction),
    __bindgen_anon_1: sdmmc_host_t__bindgen_ty_1 {
        deinit: Some(sdmmc_host_deinit),
    },
    io_int_enable: Some(sdmmc_host_io_int_enable),
    io_int_wait: Some(sdmmc_host_io_int_wait),
    command_timeout_ms: 0,
};

impl Sdmmc {
    pub fn new(base_path: &'static str) -> Result<Self, EspError> {
        let host_config = sdmmc_host_t {
            flags: 0x0001, // 1-bit
            ..DEFAULT_HOST_CONFIG
        };
        let slot_config = sdmmc_slot_config_t {
            __bindgen_anon_1: sdmmc_slot_config_t__bindgen_ty_1 {
                gpio_cd: gpio_num_t_GPIO_NUM_NC,
            },
            __bindgen_anon_2: sdmmc_slot_config_t__bindgen_ty_2 {
                gpio_wp: gpio_num_t_GPIO_NUM_NC,
            },
            width: 4,
            flags: 0,
        };
        let mount_config = esp_vfs_fat_mount_config_t {
            format_if_mount_failed: false,
            max_files: 5,
            allocation_unit_size: 0,
        };
        let mut card: *mut sdmmc_card_t = core::ptr::null_mut();
        let slot_config: *const sdmmc_slot_config_t = &slot_config;
        unsafe {
            esp!(esp_vfs_fat_sdmmc_mount(
                HeaplessCStr::new(base_path)?.ptr(),
                &host_config,
                slot_config as *const c_void,
                &mount_config,
                &mut card
            ))
        }?;
        Ok(Self { base_path, card })
    }
    pub fn info(&self) -> Option<Info> {
        let mut nclst: DWORD = 0;
        let mut fatfs: *mut FATFS = core::ptr::null_mut();
        let _status = unsafe {
            f_getfree(
                HeaplessCStr::new("0:").unwrap().ptr(),
                &mut nclst,
                &mut fatfs,
            )
        };
        unsafe {
            fatfs.as_ref().map(|fatfs| {
                let csize = fatfs.csize as usize;
                let ssize = fatfs.ssize as usize;
                let n_fatent = fatfs.n_fatent as usize;
                let free_clst = fatfs.free_clst as usize;
                Info {
                    total_bytes: csize * ssize * (n_fatent - 2),
                    free_bytes: csize * ssize * free_clst,
                }
            })
        }
    }
    pub fn open(&self, name: &str, mode: &str) -> Option<File> {
        unsafe {
            let file = fopen(
                HeaplessCStr::new_multi(&[self.base_path, "/", name])
                    .unwrap()
                    .ptr(),
                HeaplessCStr::new(mode).unwrap().ptr(),
            );
            file.as_mut().map(|file| File { file })
        }
    }
}
impl Drop for Sdmmc {
    fn drop(&mut self) {
        unsafe {
            esp_vfs_fat_sdcard_unmount(HeaplessCStr::new(self.base_path).unwrap().ptr(), self.card)
        };
    }
}
pub struct Info {
    pub total_bytes: usize,
    pub free_bytes: usize,
}
pub struct File {
    file: *mut FILE,
}
impl File {
    pub fn write(&self, data: &[u8]) -> Result<(), ()> {
        unsafe {
            match fwrite(
                data.as_ptr() as *const c_void,
                1,
                data.len() as u32,
                self.file,
            ) == data.len() as u32
            {
                true => Ok(()),
                false => Err(()),
            }
        }
    }
    pub fn read(&self, buf: &mut [u8]) -> usize {
        unsafe {
            fread(
                buf.as_mut_ptr() as *mut c_void,
                1,
                buf.len() as u32,
                self.file,
            ) as usize
        }
    }
    pub fn read_vec(&self) -> Vec<u8> {
        let mut result = Vec::new();
        loop {
            let mut buf = [0u8; 1024];
            let len = self.read(&mut buf);
            if len == 0 {
                break;
            }
            result.extend(&buf[..len]);
        }
        result
    }
}
impl Drop for File {
    fn drop(&mut self) {
        unsafe { fclose(self.file) };
    }
}

const PARTITION_LABEL_MAX_LEN: usize = 128;
struct HeaplessCStr {
    data: heapless::Vec<u8, { PARTITION_LABEL_MAX_LEN + 1 }>,
}
impl HeaplessCStr {
    fn new(s: &str) -> Result<Self, EspError> {
        let mut data = heapless::Vec::<u8, { PARTITION_LABEL_MAX_LEN + 1 }>::new();
        data.extend(s.bytes());
        data.push(0x00).map_err(|_| EspError::from(-1).unwrap())?;
        Ok(Self { data })
    }
    fn new_multi(s: &[&str]) -> Result<Self, EspError> {
        let mut data = heapless::Vec::<u8, { PARTITION_LABEL_MAX_LEN + 1 }>::new();
        for s in s {
            data.extend(s.bytes());
        }
        data.push(0x00).map_err(|_| EspError::from(-1).unwrap())?;
        Ok(Self { data })
    }
    fn ptr(&self) -> *const core::ffi::c_char {
        unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(&self.data).as_ptr() }
    }
}
