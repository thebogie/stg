# VSCode Debugging Troubleshooting

## Breakpoints Not Working

If breakpoints are not staying on or not triggering, try these fixes:

### 1. Install CodeLLDB Extension

The debugger requires the **CodeLLDB** extension:

1. Open VSCode Extensions (Ctrl+Shift+X)
2. Search for "CodeLLDB" by Vadim Chugunov
3. Install it
4. Reload VSCode

Or install via command line:
```bash
code --install-extension vadimcn.vscode-lldb
```

### 2. Clean and Rebuild

Sometimes stale build artifacts cause issues:

```bash
# Clean all build artifacts
cargo clean

# Rebuild (this will happen automatically when you start debugging)
# Or manually:
cargo build --package backend --bin backend --profile dev
```

### 3. Verify Debug Profile

The launch configuration should use the `dev` profile (which it does now). Verify:

- Open `.vscode/launch.json`
- Check that `--profile dev` is in the cargo args
- The `dev` profile in `Cargo.toml` should have `debug = true`

### 4. Check Debug Symbols

Verify debug symbols are being generated:

```bash
# Build with dev profile
cargo build --package backend --bin backend --profile dev

# Check if debug symbols exist (on Linux)
file target/debug/backend | grep "not stripped"
# Should show "not stripped" if debug symbols are present
```

### 5. Restart Debugger

1. Stop the debugger (Shift+F5)
2. Close VSCode completely
3. Reopen VSCode
4. Try debugging again

### 6. Check Breakpoint Location

Make sure you're setting breakpoints on **executable code**:
- ✅ Function bodies
- ✅ Inside `if` statements
- ✅ Inside loops
- ❌ Function signatures
- ❌ Empty lines
- ❌ Comments

### 7. Verify Source File Mapping

Sometimes the debugger can't map source files. Check:

1. Open the file where you set the breakpoint
2. Make sure it's saved
3. Set the breakpoint on a line with actual code (not blank/comment)
4. Start debugging

### 8. Check Debug Console

Look at the Debug Console for errors:
- Open Debug Console (View → Debug Console)
- Check for any error messages
- Look for "breakpoint" or "symbol" related errors

### 9. Alternative: Use `cppdbg` Type

If CodeLLDB doesn't work, you can try using the C++ debugger:

```json
{
  "type": "cppdbg",
  "request": "launch",
  "name": "Debug Backend (C++ Debugger)",
  "program": "${workspaceFolder}/target/debug/backend",
  "args": [],
  "stopAtEntry": false,
  "cwd": "${workspaceFolder}",
  "environment": [],
  "externalConsole": false,
  "MIMode": "lldb",
  "setupCommands": [
    {
      "description": "Enable pretty-printing for gdb",
      "text": "-enable-pretty-printing",
      "ignoreFailures": true
    }
  ]
}
```

### 10. Check Rust Toolchain

Make sure you have a compatible Rust toolchain:

```bash
rustc --version
# Should be 1.70+ for good debugging support

# Update if needed
rustup update stable
```

## Common Error Messages

### "Breakpoint ignored because generated code not found"

**Fix:** Clean and rebuild:
```bash
cargo clean
# Then start debugging again (will rebuild automatically)
```

### "No symbols found"

**Fix:** Ensure debug profile is used:
- Check `.vscode/launch.json` has `--profile dev`
- Verify `Cargo.toml` has `[profile.dev]` with `debug = true`

### "Unable to set breakpoint"

**Fix:** 
1. Make sure CodeLLDB extension is installed
2. Check the file is saved
3. Try setting breakpoint on a different line

## Verification Steps

After applying fixes, verify:

1. **Extension installed:**
   ```bash
   code --list-extensions | grep lldb
   # Should show: vadimcn.vscode-lldb
   ```

2. **Build with debug:**
   ```bash
   cargo build --package backend --bin backend --profile dev
   # Should complete without errors
   ```

3. **Start debugging:**
   - Press F5
   - Check Debug Console for any errors
   - Set a breakpoint in `backend/src/main.rs` on a line with code
   - The breakpoint should show as a filled red circle (not hollow)

4. **Test breakpoint:**
   - When code hits the breakpoint, execution should pause
   - Variables panel should show values
   - You can step through code (F10, F11)

## Still Not Working?

If breakpoints still don't work:

1. **Check VSCode version:** Update to latest version
2. **Check CodeLLDB version:** Update the extension
3. **Try manual debugging:**
   ```bash
   # Build
   cargo build --package backend --bin backend --profile dev
   
   # Run with lldb manually
   lldb target/debug/backend
   (lldb) breakpoint set --file main.rs --line 10
   (lldb) run
   ```

4. **Check system debugger:**
   ```bash
   which lldb
   # Should show path to lldb
   ```

5. **Report issue:** Include:
   - VSCode version
   - CodeLLDB extension version
   - Rust version (`rustc --version`)
   - OS and version
   - Error messages from Debug Console



