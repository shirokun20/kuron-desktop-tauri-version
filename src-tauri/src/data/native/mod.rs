//! Native — pindahan `kuron_native/rust` tanpa JNI (Fase 3, spec §5.1).
//! `image_ops` port penuh (task 5.1); `bubble_detector` port `BubbleDetector.kt`
//! (task 5.2, YOLO-seg via `ort` CPU); command wrapper di `commands/image_commands.rs`.

pub mod bubble_detector;
pub mod image_ops;
