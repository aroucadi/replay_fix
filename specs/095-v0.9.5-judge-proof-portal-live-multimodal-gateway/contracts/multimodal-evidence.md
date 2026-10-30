# Contract: Multimodal Evidence & Spatial Coordinate Alignment

## 1. Coordinate Systems & Geometry
- **Viewport Canvas**: Standard pixel coordinate space $[0, W) \times [0, H)$ where $(0, 0)$ represents top-left.
- **SAM 3.1 Bounding Boxes**: Normalized floating-point or integer pixel coordinates $[x_{\min}, y_{\min}, x_{\max}, y_{\max}]$.
- **Click Centroids**: Integer coordinates $(C_x, C_y)$ in CSS pixels mapped directly to viewport bounding boxes.
- **Scaling & High-DPI**: The portal must respect CSS `devicePixelRatio` when drawing canvas polygons or SVG path overlays so that masks align precisely with underlying keyframe images.

## 2. Artifact Provenance & Linkage
- Keyframes and video assets must be referenced by relative path and verified by their raw SHA-256 digest recorded in the receipt's `EvidenceLeaf`.
- If an image or video asset cannot be loaded (e.g., missing from the local folder or unbundled), the portal MUST render a high-contrast fallback banner:
  `[Artifact Not Found on Local Disk: <filename> (Expected SHA-256: <hash>)]`.
