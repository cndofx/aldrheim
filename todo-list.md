# Todo List

## Issues

- texture blending in RenderDeferredEffect is not fully working, artifacts are clearly visible on most levels
- particle hue shifts dont seem quite right either, torch embers that should be red are more orange, and some other effects that should be gold are more green-yellow

## Features

- implement PulseEmitter
- implement BiTreeNode with AdditiveEffect
- parse LavaEffect
- implement liquid/water shader
- implement "skymap" background texture
- implement skinned models
- implement animated level parts

## Optimizations

- determine if the current method of reading vertex data from a storage buffer is a bottleneck, and look into alternatives if so
  - probably parse vertex buffers on the cpu side and repack them into a consistent layout
  - consider zeux/meshoptimizer if we're preprocessing vertex data anyway