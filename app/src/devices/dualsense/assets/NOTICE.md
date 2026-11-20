# DualSense assets (not in the repository)

This folder is git-ignored except for this note. Without `model.glb` the app works normally;
the controller stage shows a placeholder instead of the 3D view.

## model.glb

"Playstation 5 Dualsense" by AHarmlessPotato, licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/):
https://sketchfab.com/3d-models/playstation-5-dualsense-878c1f882808477ab81c2fe86d5a3936

To add it: sign in to Sketchfab, open the page above, choose *Download 3D Model* →
**glTF Binary (.glb)** and save it here as `model.glb`. The app shows the credit line under the
3D view. Periph changes nothing in the file: the lightbar colour is driven through the model's
own emissive texture, and the player LEDs, mic LED, glows and callouts are drawn on top
(anchors in `controller-3d.svelte`).
