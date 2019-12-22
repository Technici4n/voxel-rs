use voxel_rs_common::debug::{DebugInfo, DebugInfoPart};

const ELEMENT_HEIGHT: i32 = 20;
const ELEMENT_OFFSET: i32 = 25;

pub fn render_debug_info(gui: &mut super::Gui, debug_info: &mut DebugInfo) {
    let debug_info = debug_info.get_debug_info();
    let x = 4;
    let mut y = 4;
    for (section, (displayed, id, messages)) in debug_info {
        let section_text = format!("{} debug info", section.to_uppercase());
        if gui.button(*id, x, y, 400, ELEMENT_HEIGHT).text(section_text, [1.0, 1.0, 1.0, 1.0]).build() {
            *displayed = !*displayed;
        }
        y += ELEMENT_OFFSET;
        if *displayed {
            for (_, part) in messages {
                match part {
                    DebugInfoPart::Message(message) => {
                        for line in message.lines() {
                            gui.text(x + 10, y, ELEMENT_HEIGHT, line.to_owned(), [1.0, 1.0, 1.0, 1.0], 0.02);
                            y += ELEMENT_HEIGHT;
                        }
                    },
                    DebugInfoPart::WorkerPerf(perf) => {
                        let text = format!(
                            "{:22} | {:6.1} ms/iter | {:5.0} iter/s | {:3.0}% efficiency | {:7} pending",
                            perf.name,
                            perf.micros_per_iter/1000.0,
                            perf.iter_per_sec,
                            perf.efficiency * 100.0,
                            perf.pending,
                        );
                        gui.text(x + 10, y, ELEMENT_HEIGHT, text, [1.0, 1.0, 1.0, 1.0], 0.02);
                        y += ELEMENT_HEIGHT;
                    },
                    DebugInfoPart::PerfBreakdown(name, breakdown) => {
                        gui.text(x + 10, y, ELEMENT_HEIGHT, format!("{} performance breakdown", name), [1.0, 1.0, 1.0, 1.0], 0.02);
                        y += ELEMENT_HEIGHT;
                        for (text, percents) in breakdown {
                            let text = format!("{:3.0}% of time: {}", *percents * 100.0, text);
                            gui.text(x + 20, y, ELEMENT_HEIGHT, text, [1.0, 1.0, 1.0, 1.0], 0.02);
                            y += ELEMENT_HEIGHT;
                        }
                    },
                }
            }
        }
    }
}