# Modpack Features Implementation

This document outlines the modpack features implemented as requested in the issue.

## 1. Modpack Updates and Password Protection

### Backend Implementation
- Added `update_modpack_instance` Tauri command in `src-tauri/src/core/instance_manager.rs`
- Supports password validation for protected modpacks
- Validates instance existence before proceeding
- Emits status events to frontend during update process

### Frontend Integration
- Updated `ModpackInstallButton.tsx` to use new `update_modpack_instance` command
- Existing password dialog integration maintained
- Error handling for invalid passwords

## 2. "Latest" Version Handling

### Data Structure Updates
- Enhanced `ModpackInfo` struct to include `modpackVersionId` field
- Added TypeScript interface definition for `ModpackInfo`
- Support for "latest" string value to trigger automatic updates

### Logic Implementation
- `update_modpack_instance` checks if `modpackVersionId` is "latest"
- Automatically initiates update process for "latest" versions
- Placeholder for API calls to check for newer versions

## 3. Modpack Asset Validation

### Core Validation Function
- Added `validate_modpack_assets` method to `InstanceBootstrap` struct
- Reads `modpack_manifest.json` from instance directory
- Validates files against manifest (path, size, hash)
- Reports missing or corrupted files
- Emits progress events during validation

### Integration Points
- Added `validate_modpack_assets` Tauri command for manual validation
- Integrated into pre-launch process in `instance_launcher.rs`
- Runs automatically before game launch for modpack instances

## 4. New Prelaunch State: "downloading-modpack-assets"

### Frontend State Management
- Added new status to `InstanceState` type in `InstancesContext.tsx`
- Added event listener for `instance-downloading-modpack-assets`
- Updated `PreLaunchInstance.tsx` to handle new state

### Backend Event Emission
- `validate_modpack_assets` emits status events
- `update_modpack_instance` uses new state during updates
- Consistent with existing prelaunch state pattern

### UI Behavior
- New state behaves identically to existing states
- Shows loading indicator and progress messages
- Prevents game launch during modpack asset processing

## 5. Key Files Modified

### Rust Backend
- `src-tauri/src/core/minecraft_instance.rs` - Added modpackVersionId to ModpackInfo
- `src-tauri/src/core/models.rs` - Updated ModpackInfo struct
- `src-tauri/src/core/instance_manager.rs` - Added update_modpack_instance command
- `src-tauri/src/core/instance_bootstrap.rs` - Added validate_modpack_assets method
- `src-tauri/src/core/instance_launcher.rs` - Integrated modpack validation into launch process
- `src-tauri/src/main.rs` - Registered new Tauri commands

### React Frontend
- `src/types/TauriCommandReturns.d.ts` - Added TypeScript interfaces
- `src/stores/InstancesContext.tsx` - Added new state and event listener
- `src/views/PreLaunchInstance.tsx` - Updated to handle new state
- `src/components/install-modpacks/ModpackInstallButton.tsx` - Updated to use new command

## 6. Usage Examples

### Modpack with Latest Version
```json
{
  "instanceId": "test-instance",
  "modpackInfo": {
    "name": "ExamplePack",
    "version": "1.0.0",
    "modpackVersionId": "latest"
  }
}
```

### Modpack Manifest Structure
```json
{
  "files": [
    {
      "path": "mods/example-mod.jar",
      "hash": "sha256:abc123...",
      "size": 1024576
    }
  ]
}
```

### Frontend Update Call
```typescript
await invoke("update_modpack_instance", {
    instanceId: "test-instance",
    modpackId: "example-modpack",
    password: "optional-password"
});
```

## 7. Notes

- Password validation is currently simulated and needs API integration
- Hash validation in asset checking is placeholder for future implementation
- File download for missing assets is marked as TODO for future implementation
- All functionality follows existing patterns to maintain consistency
- TypeORM entities are avoided as requested (using existing structures instead)