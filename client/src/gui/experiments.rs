use voxel_rs_common::debug::DebugInfo;

const ELEMENT_HEIGHT: i32 = 20;
const ELEMENT_OFFSET: i32 = 25;

pub fn render_debug_info(gui: &mut super::Gui, debug_info: &mut DebugInfo) {
    let debug_info = debug_info.get_debug_info();
    let x = 4;
    let mut y = 4;
    for (section, (displayed, id, messages)) in debug_info {
        gui.text(x, y, ELEMENT_HEIGHT, format!("{} debug info", section.to_uppercase()), [1.0, 1.0, 1.0, 1.0], 0.03);
        if gui.button(*id, x, y, 400, ELEMENT_HEIGHT) {
            *displayed = !*displayed;
        }
        y += ELEMENT_OFFSET;
        if *displayed {
            for (_, message) in messages {
                for line in message.lines() {
                    gui.text(x + 10, y, ELEMENT_HEIGHT, line.to_owned(), [1.0, 1.0, 1.0, 1.0], 0.02);
                    y += ELEMENT_OFFSET;
                }
            }
        }
    }
}