# Git Credentials Setup for Cursor

## Using Both SSH and Windows Git Credential Manager

**Yes, you can use both!** You can:
- Use **SSH** for some repositories (faster, no prompts)
- Use **HTTPS with Windows Credential Manager** for others (easier for some workflows)
- Even use different methods for different remotes in the same repo

### How It Works

Git automatically chooses the authentication method based on the remote URL:
- **SSH URLs** (`git@github.com:user/repo.git`) → Uses SSH keys
- **HTTPS URLs** (`https://github.com/user/repo.git`) → Uses credential manager

### Example: Mixed Setup

```bash
# Check your current remote
git remote -v

# If it's HTTPS, you can keep it (uses Windows Credential Manager)
# Or switch to SSH:
git remote set-url origin git@github.com:username/repo.git

# You can even have multiple remotes with different methods:
git remote add github-ssh git@github.com:user/repo.git
git remote add github-https https://github.com/user/repo.git
```

### Best Practice

- **SSH for development repos** (faster, no prompts, better for automation)
- **HTTPS for occasional clones** (easier, works everywhere)

Both methods can coexist - Git will use whichever matches your remote URL.

---

## Quick Fix: Configure Git Credentials

### Option 1: Use Git Credential Manager (Windows/WSL)

If you're on Windows/WSL and using Git Credential Manager:

```bash
# Check if credential manager is configured
git config --global credential.helper

# If not set, configure it:
git config --global credential.helper manager-core

# Or for WSL specifically:
git config --global credential.helper "/mnt/c/Program\ Files/Git/mingw64/bin/git-credential-manager.exe"
```

### Option 2: Use SSH Keys (Recommended for GitHub/GitLab)

1. **Generate SSH key** (if you don't have one):
```bash
ssh-keygen -t ed25519 -C "your.email@example.com"
```

2. **Add to SSH agent**:
```bash
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
```

3. **Add public key to GitHub/GitLab**:
```bash
cat ~/.ssh/id_ed25519.pub
# Copy the output and add it to your GitHub/GitLab account
```

4. **Update git remote to use SSH**:
```bash
git remote set-url origin git@github.com:username/repo.git
```

### Option 3: Use Personal Access Token

1. **Create a token** on GitHub/GitLab
2. **Use it as password** when git prompts for credentials
3. **Or store it**:
```bash
git config --global credential.helper store
# Then on next git operation, enter username and token as password
```

### Option 4: Configure in Cursor Settings

1. Open Cursor Settings (`Ctrl+,`)
2. Search for "git"
3. Set these:
   - `git.terminalAuthentication`: `false`
   - `git.useEditorAsCommitInput`: `true`
   - `git.allowNoVerifyCommit`: `true`

### Option 5: Skip Pre-commit Hook (Temporary)

If you need to commit urgently:
```bash
git commit --no-verify -m "your message"
```

## Troubleshooting

### If Git Credential Manager isn't working in WSL:

```bash
# Try using Windows credential manager from WSL
git config --global credential.helper "/mnt/c/Program\ Files/Git/mingw64/bin/git-credential-manager.exe"

# Or use cache (stores credentials for 15 minutes)
git config --global credential.helper cache
```

### If you see "failed to authenticate in git mode":

This usually means:
1. Git is trying to authenticate but credentials aren't configured
2. The pre-commit hook is triggering git operations

**Solution**: The pre-commit hook has been updated to prevent git auth prompts. If issues persist:
- Configure git credentials using one of the options above
- Or use `git commit --no-verify` to skip the hook temporarily
