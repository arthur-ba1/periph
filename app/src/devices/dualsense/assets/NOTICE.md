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

## Emulation previews

Save the optional glTF Binary downloads in this folder with the filenames below. The
emulation selector shows these models with Periph's translucent shading and their original
button markings. The stage shows a placeholder when the selected asset is missing.

| Filename | Model | License |
| --- | --- | --- |
| `dualshock4.glb` | [DualShock 4 PlayStation Controller](https://sketchfab.com/3d-models/dualshock-4-playstation-controller-e3c2f0dc16524fc19cdde45bad1de1a9) by shaielwolf | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |
| `xbox360.glb` | [XBox 360 controller](https://sketchfab.com/3d-models/xbox-360-controller-31f699500e58427ebf042ff75abd4bec) by Carlos Nava | [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) |
