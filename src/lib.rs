use anyhow::{Result, anyhow};
use embedded_storage::{
    ReadStorage, Storage,
    nor_flash::{
        ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash, check_erase,
        check_read, check_write,
    },
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use thiserror::Error;

// ------------------------------
// 1. 错误类型定义（实现NorFlashError）
// ------------------------------
#[derive(Debug, Error)]
pub enum FlashMockError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Write to non-erased area (offset: {offset})")]
    WriteToNonErased { offset: u32 },
    #[error("NOR flash check failed: {0:?}")]
    CheckFailed(NorFlashErrorKind),
}

impl NorFlashError for FlashMockError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            FlashMockError::Io(_) => NorFlashErrorKind::Other,
            FlashMockError::WriteToNonErased { .. } => NorFlashErrorKind::Other,
            FlashMockError::CheckFailed(kind) => *kind,
        }
    }
}

// ------------------------------
// 2. FlashMock结构体（用const泛型定义静态参数）
// ------------------------------
/// const泛型说明：
/// - READ_SIZE: 最小读取单位（编译时确定，需是2的幂）
/// - WRITE_SIZE: 最小写入单位（编译时确定，需是2的幂）
/// - ERASE_SIZE: 最小擦除单位（编译时确定，需是2的幂）
pub struct FlashMock<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> {
    _path: String,         // 持久化文件路径
    total_capacity: usize, // 总存储容量（需是ERASE_SIZE的整数倍）
    file: File,            // 文件句柄
}

impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize>
    FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    /// 创建模拟NOR Flash实例
    /// - `path`: 持久化文件路径
    /// - `total_capacity`: 总存储容量（必须是ERASE_SIZE的整数倍）
    pub fn new<P: AsRef<Path>>(path: P, total_capacity: usize) -> Result<Self> {
        // 编译时验证核心参数（2的幂 + 容量倍数）
        if (READ_SIZE & (READ_SIZE - 1)) != 0 {
            return Err(anyhow!(
                "READ_SIZE must be a power of two (got {READ_SIZE})"
            ));
        }
        if (WRITE_SIZE & (WRITE_SIZE - 1)) != 0 {
            return Err(anyhow!(
                "WRITE_SIZE must be a power of two (got {WRITE_SIZE})"
            ));
        }
        if (ERASE_SIZE & (ERASE_SIZE - 1)) != 0 {
            return Err(anyhow!(
                "ERASE_SIZE must be a power of two (got {ERASE_SIZE})"
            ));
        }
        if total_capacity % ERASE_SIZE != 0 {
            return Err(anyhow!(
                "Total capacity must be multiple of ERASE_SIZE ({} % {} != 0)",
                total_capacity,
                ERASE_SIZE
            ));
        }

        // 处理文件路径
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid path: cannot convert to string"))?
            .to_string();

        // 初始化文件（不存在则创建并填充0xFF）
        let file = if !Path::new(&path).exists() {
            let mut file = File::create(&path)?;
            let erase_block = vec![0xFFu8; ERASE_SIZE];
            for _ in 0..(total_capacity / ERASE_SIZE) {
                file.write_all(&erase_block)?;
            }
            file
        } else {
            File::options().read(true).write(true).open(&path)?
        };

        Ok(Self {
            _path: path,
            total_capacity,
            file,
        })
    }

    /// 检查目标区域是否已擦除（全为0xFF）
    fn is_area_erased(&mut self, offset: u32, length: usize) -> Result<bool, FlashMockError> {
        let mut buffer = vec![0u8; length];
        self.file.seek(SeekFrom::Start(offset as u64))?;
        self.file.read_exact(&mut buffer)?;
        Ok(buffer.iter().all(|&byte| byte == 0xFF))
    }
}

// ------------------------------
// 3. 实现ErrorType（关联错误类型）
// ------------------------------
impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> ErrorType
    for FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    type Error = FlashMockError;
}

// ------------------------------
// 4. 实现ReadNorFlash（定义READ_SIZE关联常量）
// ------------------------------
impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> ReadNorFlash
    for FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    /// 关联常量：最小读取单位（从const泛型获取，编译时确定）
    const READ_SIZE: usize = READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        // 复用库函数检查参数（对齐 + 边界）
        check_read(self, offset, bytes.len()).map_err(FlashMockError::CheckFailed)?;

        // 执行文件读取
        self.file.seek(SeekFrom::Start(offset as u64))?;
        self.file.read_exact(bytes)?;
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.total_capacity
    }
}

// ------------------------------
// 5. 实现NorFlash（定义WRITE_SIZE/ERASE_SIZE关联常量）
// ------------------------------
impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> NorFlash
    for FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    /// 关联常量：最小写入单位（从const泛型获取）
    const WRITE_SIZE: usize = WRITE_SIZE;
    /// 关联常量：最小擦除单位（从const泛型获取）
    const ERASE_SIZE: usize = ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        // 复用库函数检查参数（from<=to + 对齐 + 边界）
        check_erase(self, from, to).map_err(FlashMockError::CheckFailed)?;

        // 填充0xFF模拟擦除
        let erase_length = (to - from) as usize;
        if erase_length > 0 {
            self.file.seek(SeekFrom::Start(from as u64))?;
            self.file.write_all(&vec![0xFFu8; erase_length])?;
            self.file.flush()?;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        // 复用库函数检查参数（对齐 + 边界）
        check_write(self, offset, bytes.len()).map_err(FlashMockError::CheckFailed)?;

        // 验证目标区域已擦除（NOR Flash核心约束）
        if !self.is_area_erased(offset, bytes.len())? {
            return Err(FlashMockError::WriteToNonErased { offset });
        }

        // 执行文件写入
        self.file.seek(SeekFrom::Start(offset as u64))?;
        self.file.write_all(bytes)?;
        self.file.flush()?;
        Ok(())
    }
}

// ------------------------------
// 6. 实现ReadStorage（兼容上层只读接口）
// ------------------------------
impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> ReadStorage
    for FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    type Error = FlashMockError;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        ReadNorFlash::read(self, offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.total_capacity
    }
}

// ------------------------------
// 7. 实现Storage（兼容上层读写接口，自动擦除）
// ------------------------------
impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> Storage
    for FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        // 复用库的Rmw逻辑，自动处理“读-擦-写”流程
        let mut merge_buffer = vec![0xFFu8; ERASE_SIZE];
        let mut rmw_storage =
            embedded_storage::nor_flash::RmwNorFlashStorage::new(self, &mut merge_buffer);
        rmw_storage.write(offset, bytes)
    }
}

// ------------------------------
// 8. Drop trait（确保数据持久化）
// ------------------------------
impl<const READ_SIZE: usize, const WRITE_SIZE: usize, const ERASE_SIZE: usize> Drop
    for FlashMock<READ_SIZE, WRITE_SIZE, ERASE_SIZE>
{
    fn drop(&mut self) {
        let _ = self.file.sync_all(); // 同步文件到磁盘
    }
}
