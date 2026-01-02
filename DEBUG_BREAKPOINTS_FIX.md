# Fix: Breakpoints Not Working in VSCode

## The Problem

Breakpoints not staying on or not triggering in VSCode debugger.

## The Solution

### Step 1: Install CodeLLDB Extension (REQUIRED)

The debugger requires the **CodeLLDB** extension. Install it:

**Option A: Via VSCode UI**
1. Press `Ctrl+Shift+X` (or `Cmd+Shift+X` on Mac) to open Extensions
2. Search for "CodeLLDB"
3. Install "CodeLLDB" by Vadim Chugunov
4. Reload VSCode when prompted

**Option B: Via Command Line**
```bash
code --install-extension vadimcn.vscode-lldb
```

**Option C: Via Extensions File**
The extension is already listed in `.vscode/extensions.json` as recommended. VSCode should prompt you to install it, or:
1. Press `Ctrl+Shift+P`
2. Type "Extensions: Show Recommended Extensions"
3. Click "Install All"

### Step 2: Clean and Rebuild

After installing the extension:

```bash
# Clean old build artifacts
cargo clean

# The debugger will rebuild automatically when you start it
# Or rebuild manually:
cargo build --package backend --bin backend --profile dev
```

### Step 3: Restart VSCode

1. Close VSCode completely
2. Reopen VSCode
3. Try debugging again

### Step 4: Verify Breakpoints Work

1. Open `backend/src/main.rs`
2. Set a breakpoint on a line with actual code (not blank/comment)
3. Press **F5** to start debugging
4. Select "Debug Backend (Hybrid Dev)"
5. The breakpoint should:
   - Show as a **filled red circle** (not hollow)
   - Execution should **pause** when it hits
   - Variables panel should show values

## What Was Fixed

1. ✅ Updated `.vscode/launch.json` to explicitly use `--profile dev` for debug symbols
2. ✅ Added `RUST_BACKTRACE=full` for better debugging
3. ✅ Created troubleshooting guide: `docs/DEBUGGING_TROUBLESHOOTING.md`

## Still Not Working?

See the complete troubleshooting guide:
- `docs/DEBUGGING_TROUBLESHOOTING.md` - Full troubleshooting steps

Common additional fixes:
- Make sure breakpoint is on executable code (not function signature)
- Check Debug Console for error messages
- Verify `Cargo.toml` has `[profile.dev]` with `debug = true` (it does)

## Quick Test

After installing CodeLLDB:

```bash
# 1. Clean
cargo clean

# 2. Start debugging in VSCode (F5)
#    This will rebuild automatically

# 3. Set breakpoint in backend/src/main.rs
#    Should work now!
```



