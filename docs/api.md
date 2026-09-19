# Rocktier PDF Squeeze API Specification

## Tauri Commands

### compress_pdf

Compress a single PDF file.

**Input:**
```ts
interface CompressInput {
  inputPath: string;
  profile: 'web' | 'balanced' | 'archive' | 'custom';
  outputSuffix?: string; // default: '_compressed'
}
```

**Output:**
```ts
interface CompressOutput {
  success: boolean;
  outputPath: string;
  outputSize?: number;
  inputSize?: number;
  profile: string;
}
```

**Errors:**
- `Invalid input path` — path is empty or has no parent
- `Invalid file name` — path has no file stem
- `Failed to start compression engine` — Go binary not found or crashed
- `Compression failed` — pdfcpu returned error (stderr forwarded)

## Go Engine CLI

### compress

```bash
pdf-engine compress <input> <output> --profile <name>
```

**Profiles:**
- `web` — 150 DPI, JPEG Q=75, RGB, strip metadata/thumbnails
- `balanced` — 200 DPI, JPEG Q=80, RGB, keep metadata
- `archive` — 300 DPI, JPEG Q=85, keep original, keep metadata
- `custom` — pdfcpu defaults

**Output (stdout):**
```json
{
  "success": true,
  "input": "/path/to/input.pdf",
  "output": "/path/to/output.pdf",
  "outputSize": 12345,
  "profile": "balanced"
}
```

**Errors (stderr):**
```
Error: <pdfcpu error message>
```

Exit code: `0` on success, `1` on failure.

## Frontend State Machine

```
idle → compressing → done
               → error
```

- `idle`: file selected, not yet processed
- `compressing`: engine is running
- `done`: compression succeeded, `compressedSize` is set
- `error`: compression failed, `error` message is set

## File Path Handling

- **macOS**: `/Users/username/Desktop/file.pdf`
- **Windows**: `C:\Users\username\Desktop\file.pdf`

Tauri's `invoke` passes strings as-is. Go engine receives raw file system paths.

## Profile Schema (Frontend)

```ts
interface Profile {
  name: string;
  dpi: number;
  quality: number;
  downsampleTo: number;
  downsampleThreshold: number;
  colorConversion: 'RGB' | 'Keep';
  removeAnnotations: boolean;
  removeThumbnails: boolean;
  removeMetadata: boolean;
  fontSubsetting: boolean;
}
```

Frontend profiles are informational only. Go engine defines the actual pdfcpu configuration.
