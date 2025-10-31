# Snapshot Format Migration Guide

## Changes in v0.3.2

Starting from version 0.3.2, snapshots now store **relative paths** instead of absolute paths.

### Why This Change?

Previously, snapshots contained absolute file paths like:
```json
{
  "file_path": "/Users/username/project/src/app/component.ts"
}
```

This caused issues when comparing snapshots between different environments (e.g., local machine vs CI):
- Local: `/Users/username/project/src/...`
- CI: `/home/runner/work/project/src/...`

### New Format

Snapshots now store paths relative to `analyzeDirectory`:
```json
{
  "file_path": "app/component.ts"
}
```

### Migration

**If you have existing snapshots with absolute paths:**

1. **Regenerate your snapshots** using the new version:
   ```bash
   sca --config-file config.json --generate-snapshot snapshot.json
   ```

2. The new snapshot will automatically use relative paths and work across all environments.

### Backwards Compatibility

- Old snapshots with absolute paths will still work if your environment paths haven't changed
- For portability, regenerate snapshots after upgrading to v0.3.2+

### Benefits

✅ Snapshots are now portable between different machines  
✅ CI/CD pipelines work without path-specific issues  
✅ Team members can share snapshots regardless of their local directory structure
