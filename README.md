# embedded-storage-std-mock

A `std`-based mock implementation of the `embedded-storage` crate, using a file to simulate embedded NOR Flash chips. Designed for PC-side development and testing of embedded storage logic—no real hardware required.


## 🌟 Project Overview
This library implements the `embedded-storage` trait hierarchy (including `ReadNorFlash`, `NorFlash`, `ReadStorage`, and `Storage`) using a **local file** as the backing store. It accurately simulates key NOR Flash behaviors (e.g., "erase-before-write", alignment constraints) while persisting data between program runs.

Perfect for:
- Testing embedded storage logic on a PC (no hardware needed)
- Validating read/write/erase workflows before deploying to hardware
- Debugging storage-related bugs in a familiar desktop environment


## 🚀 Features
1. **Compliant with `embedded-storage` Spec**: Implements all required traits from `embedded-storage`'s `nor_flash` module.
2. **NOR Flash Behavior Simulation**:
   - Enforces "erase-before-write" (cannot write to non-erased regions).
   - Respects alignment constraints (read/write/erase sizes must be powers of 2).
   - Erased regions are filled with `0xFF` (matching real NOR Flash).
3. **File Persistence**: Uses a local file to store mock Flash data—data survives program restarts.
4. **Automatic Erase for `Storage` Trait**: Implements `Storage` with auto-erase (via `RmwNorFlashStorage` from `embedded-storage`), simplifying upper-layer usage.
5. **Compile-Time Validation**: Uses `const` generics to enforce valid Flash parameters (e.g., power-of-2 sizes) at compile time.


## 📦 Installation
Add to your `Cargo.toml`:
```toml
[dependencies]
embedded-storage = "0.1.1"          # Required trait definitions
embedded-storage-std-mock = "0.1.0" # This mock library
```


## ⚡ Quick Start
Here’s a complete example demonstrating how to create a mock Flash, erase, write, and read data:

```rust
use anyhow::Result;
use embedded_storage_std_mock::FlashMock;

fn main() -> Result<()> {
    // 1. Create a mock NOR Flash:
    //    - Read size: 1 byte (const generic)
    //    - Write size: 1 byte (const generic)
    //    - Erase size: 4096 bytes (const generic, typical sector size)
    //    - Backing file: "./mock_flash.bin"
    //    - Total capacity: 32768 bytes (8 sectors × 4096 bytes)
    let mut flash = FlashMock::<1, 1, 4096>::new("./mock_flash.bin", 32768)?;

    println!(
        "Mock Flash Initialized:\n\
        Total Capacity: {} bytes\n\
        Read Size: {} byte\n\
        Write Size: {} byte\n\
        Erase Size: {} bytes",
        flash.capacity(),
        FlashMock::<1, 1, 4096>::READ_SIZE,
        FlashMock::<1, 1, 4096>::WRITE_SIZE,
        FlashMock::<1, 1, 4096>::ERASE_SIZE
    );

    // 2. Erase the first sector (addresses 0 → 4095)
    flash.erase(0, 4096)?;
    println!("\nErased sector 0 (0–4095 bytes)");

    // 3. Write data to address 0x100 (256 in decimal)
    let write_data = b"Hello, embedded-storage!";
    flash.write(0x100, write_data)?;
    println!("Wrote data to 0x100: {:?}", write_data);

    // 4. Read back the data to verify
    let mut read_buffer = vec![0u8; write_data.len()];
    flash.read(0x100, &mut read_buffer)?;
    assert_eq!(read_buffer, write_data);
    println!("Read data from 0x100: {:?} (match: {})", read_buffer, read_buffer == write_data);

    // 5. Use Storage trait (auto-erase, no manual erase needed)
    let auto_write_data = b"Auto-erase works!";
    flash.write(0x200, auto_write_data)?; // Storage::write auto-erases required sectors
    println!("\nAuto-wrote data to 0x200: {:?}", auto_write_data);

    Ok(())
}
```

### Run the Example
1. Save the code to `src/main.rs`.
2. Run with `cargo run`.
3. Check `./mock_flash.bin`—it will persist the mock Flash data between runs.


## ⚠️ Important Notes
1. **File Persistence**: The mock Flash file (e.g., `./mock_flash.bin`) persists between program runs. Delete it to reset the mock Flash to its initial state (all `0xFF`).
2. **Alignment Constraints**: Real NOR Flash enforces alignment—this library mirrors that. Ensure:
   - `read`/`write` offsets are aligned to `READ_SIZE`/`WRITE_SIZE`.
   - `erase` ranges are aligned to `ERASE_SIZE`.
3. **Performance**: File I/O is slower than real Flash. This library is for testing, not production use.
4. **Error Handling**: Errors (e.g., misalignment, out-of-bounds access) are returned as `FlashMockError`, which implements `embedded_storage::nor_flash::NorFlashError`.


## ❓ Frequently Asked Questions (FAQ)

### Q: Why do I get an "alignment error"?
A: You violated the alignment constraints. For example:
- Writing to an offset not divisible by `WRITE_SIZE`.
- Erasing a range not divisible by `ERASE_SIZE`.
Fix: Ensure offsets/ranges are aligned to the corresponding size (use `const` generics to enforce valid sizes at compile time).


### Q: Why does `write` fail with "Write to non-erased area"?
A: NOR Flash requires erasing before writing. You must call `erase` on the target sector first, or use the `Storage` trait’s auto-erase `write` method.


### Q: How do I reset the mock Flash data?
A: Delete the backing file (e.g., `./mock_flash.bin`). The next time you run the program, a new file will be created with all bytes set to `0xFF` (erased state).


## 📄 License
This project is licensed under the **MIT License**—see the [LICENSE](LICENSE) file for details.


## 🤝 Contributing
Contributions are welcome! Feel free to open issues for bugs, feature requests, or submit pull requests with improvements.