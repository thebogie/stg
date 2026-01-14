# Locale Fix for VSCode Git Operations

## Problem
VSCode git operations were failing with:
```
/bin/bash: warning setlocale: LC_ALL: cannot change locale (en_US.UTF-8)
```

## Root Cause
The system only has these locales available:
- `C`
- `C.utf8` (lowercase)
- `POSIX`

But VSCode/git was trying to use `en_US.UTF-8` or `C.UTF-8` (uppercase), which don't exist.

## Fixes Applied

### 1. Updated VSCode Settings
Updated `.vscode/settings.json` to:
- Set `LC_ALL=C.utf8` and `LANG=C.utf8` for terminal
- Disabled locale auto-detection: `"terminal.integrated.detectLocale": "off"`

### 2. Fixed Shell Profiles
Updated `~/.bashrc`, `~/.bash_profile`, and `~/.profile` to use `C.utf8` instead of `C.UTF-8`.

### 3. Updated Pre-commit Hook
The `.git/hooks/pre-commit` hook now:
- Sets locale to `C.utf8` before running cargo
- Filters out locale warnings from cargo output

## Next Steps

1. **Restart VSCode completely** (close all windows and reopen)
   - This ensures VSCode picks up the new settings

2. **If the error persists**, you may need to:
   ```bash
   # Option A: Generate the missing locale (requires sudo)
   sudo locale-gen en_US.UTF-8
   sudo update-locale LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8
   
   # Option B: Use the locale fix script
   # Set git to use the wrapper (in VSCode settings):
   # "git.path": "/home/thebogie/work/stg/.vscode/fix-locale.sh /usr/bin/git"
   ```

3. **Test the fix**:
   - Try committing and pushing from VSCode
   - The locale error should be gone

## Files Modified
- `.vscode/settings.json` - Added locale environment variables
- `~/.bashrc`, `~/.bash_profile`, `~/.profile` - Fixed locale to use `C.utf8`
- `.git/hooks/pre-commit` - Enhanced locale handling

## Note
The system locale file `/etc/default/locale` still has `LANG=C.UTF-8` (uppercase), but this shouldn't cause issues if VSCode settings override it. If problems persist, you may need to update that file (requires sudo).
