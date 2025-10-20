# embedded-storage-std-mock

基于 `std` 实现的 `embedded-storage` 模拟库，使用本地文件模拟嵌入式 NOR Flash 芯片。适用于 PC 端开发和测试嵌入式存储逻辑，无需依赖真实硬件。


## 🌟 项目概述
本库实现了 `embedded-storage` Trait 体系（包括 `ReadNorFlash`、`NorFlash`、`ReadStorage` 和 `Storage`），并以**本地文件**作为后端存储。它能精准模拟 NOR Flash 的核心行为（如“先擦后写”、对齐约束），同时支持程序重启后数据持久化。

非常适合以下场景：
- 在 PC 端测试嵌入式存储逻辑（无需硬件）
- 部署到硬件前验证读写擦除流程
- 在熟悉的桌面环境中调试存储相关问题


## 🚀 核心特性
1. **符合 `embedded-storage` 规范**：完整实现 `embedded-storage` 中 `nor_flash` 模块的所有必需 Trait。
2. **NOR Flash 行为模拟**：
   - 强制“先擦后写”规则（不能向未擦除区域写入数据）
   - 遵循对齐约束（读/写/擦除大小必须是 2 的幂）
   - 擦除后的区域用 `0xFF` 填充（与真实 NOR Flash 一致）
3. **文件持久化**：使用本地文件存储模拟 Flash 数据，程序重启后数据不丢失。
4. **`Storage` Trait 自动擦除**：实现 `Storage` Trait 并支持自动擦除（基于 `embedded-storage` 的 `RmwNorFlashStorage`），简化上层使用。
5. **编译时参数校验**：通过 `const` 泛型强制 Flash 参数合法性（如 2 的幂大小），错误提前暴露。


## 📦 安装
在你的 `Cargo.toml` 中添加依赖：
```toml
[dependencies]
embedded-storage = "0.1.1"          # 必需的 Trait 定义
embedded-storage-std-mock = "0.1.0" # 本模拟库
```


## ⚡ 快速开始
以下是完整示例，展示如何创建模拟 Flash、执行擦除、写入和读取操作：

```rust
use anyhow::Result;
use embedded_storage_std_mock::FlashMock;

fn main() -> Result<()> {
    // 1. 创建模拟 NOR Flash：
    //    - 读取大小：1 字节（const 泛型参数）
    //    - 写入大小：1 字节（const 泛型参数）
    //    - 擦除大小：4096 字节（const 泛型参数，典型扇区大小）
    //    - 后端文件：./mock_flash.bin
    //    - 总容量：32768 字节（8 个扇区 × 4096 字节）
    let mut flash = FlashMock::<1, 1, 4096>::new("./mock_flash.bin", 32768)?;

    println!(
        "模拟 Flash 初始化完成：\n\
        总容量：{} 字节\n\
        读取单位：{} 字节\n\
        写入单位：{} 字节\n\
        擦除单位：{} 字节",
        flash.capacity(),
        FlashMock::<1, 1, 4096>::READ_SIZE,
        FlashMock::<1, 1, 4096>::WRITE_SIZE,
        FlashMock::<1, 1, 4096>::ERASE_SIZE
    );

    // 2. 擦除第一个扇区（地址 0 → 4095）
    flash.erase(0, 4096)?;
    println!("\n已擦除扇区 0（0–4095 字节）");

    // 3. 向地址 0x100（十进制 256）写入数据
    let write_data = b"Hello, embedded-storage!";
    flash.write(0x100, write_data)?;
    println!("向 0x100 地址写入数据：{:?}", write_data);

    // 4. 读取数据并验证
    let mut read_buffer = vec![0u8; write_data.len()];
    flash.read(0x100, &mut read_buffer)?;
    assert_eq!(read_buffer, write_data);
    println!("从 0x100 地址读取数据：{:?}（匹配：{}）", read_buffer, read_buffer == write_data);

    // 5. 使用 Storage Trait（自动擦除，无需手动擦除）
    let auto_write_data = b"Auto-erase works!";
    flash.write(0x200, auto_write_data)?; // Storage::write 会自动擦除所需扇区
    println!("\n自动擦除后向 0x200 写入数据：{:?}", auto_write_data);

    Ok(())
}
```

### 运行示例
1. 将上述代码保存到 `src/main.rs`。
2. 执行 `cargo run` 运行程序。
3. 查看 `./mock_flash.bin` 文件，该文件会在程序重启后保留模拟 Flash 数据。


## ⚠️ 重要说明
1. **文件持久化**：模拟 Flash 的数据文件（如 `./mock_flash.bin`）会在程序间保留。若需重置 Flash 到初始状态（全 `0xFF`），删除该文件即可。
2. **对齐约束**：真实 NOR Flash 会强制对齐要求，本库完全模拟这一特性：
   - `read`/`write` 操作的地址需分别对齐到 `READ_SIZE`/`WRITE_SIZE`。
   - `erase` 操作的地址范围需对齐到 `ERASE_SIZE`。
3. **性能提示**：文件 I/O 速度慢于真实 Flash，本库仅用于测试，不适合生产环境。
4. **错误处理**：错误（如对齐失败、地址越界）会以 `FlashMockError` 返回，该类型已实现 `embedded_storage::nor_flash::NorFlashError`。


## ❓ 常见问题（FAQ）

### Q：为什么会报“对齐错误”？
A：你违反了对齐约束，例如：
- 写入地址不是 `WRITE_SIZE` 的整数倍。
- 擦除范围不是 `ERASE_SIZE` 的整数倍。
解决方案：确保地址/范围对齐到对应的单位，或通过 `const` 泛型在编译时强制合法参数。


### Q：为什么 `write` 会报“向未擦除区域写入”错误？
A：NOR Flash 硬件要求“先擦后写”。你需要先调用 `erase` 擦除目标扇区，或直接使用 `Storage` Trait 的自动擦除 `write` 方法。


### Q：如何重置模拟 Flash 的数据？
A：删除后端数据文件（如 `./mock_flash.bin`）即可。下次程序运行时会自动创建新文件，并以全 `0xFF` 填充（模拟初始擦除状态）。


## 📄 许可证
本项目基于 **MIT 许可证** 开源，详见 [LICENSE](LICENSE) 文件。


## 🤝 贡献指南
欢迎参与贡献！如有 Bug 反馈、功能需求，可直接提交 Issue；也可开发新功能后提交 Pull Request。