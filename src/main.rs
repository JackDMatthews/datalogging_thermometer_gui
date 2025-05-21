use eframe::egui;
use std::{sync::{Arc, Mutex}, thread};

const NUM_CHANNELS: usize = 8;
const AUTOSAVE_SECONDS_INTERVAL: u64 = 60;

#[derive(Clone)]
struct Channel {
    data: Vec<(u64, Option<f64>)>, // (Timestamp, temperature)
    enabled: bool,
    colour: egui::Color32,
}
#[derive(Clone)]
struct ThermometerApp{
    channels : Arc<Mutex< [Channel; NUM_CHANNELS] >>, // data from the serial port
    timestamp_datetime: Arc<Mutex< Vec<(u64, String)> >>, // timestamp and equivalent datetime
    port_names: Vec<String>, // list of available serial ports
    selected_port_name: Arc<Mutex<String>>, // selected serial port
}

impl ThermometerApp {
    // TODO: Make safe from panics. This is called from a different thread so the application will just keep going.
    fn save_to_csv(&self) {
        // write the data to a .csv file
        let current_time = chrono::Local::now().format("%Y-%m-%d %H-%M-%S").to_string();

        let mut writer = csv::Writer::from_path(format!("data {}.csv", current_time)).unwrap();

        let sensor_headers: Vec<String> = (1..=NUM_CHANNELS).map(|i| format!("Sensor {}", i)).collect();

        let mut headers = vec!["Time since start (ms)", "datetime of data"];
        headers.extend(sensor_headers.iter().map(|s| s.as_str()));
        writer.write_record(&headers).unwrap();

        let channels: &[Channel; NUM_CHANNELS] = &self.channels.lock().unwrap();
        let timestamp_datetime = &self.timestamp_datetime.lock().unwrap();

        for (i, (timestamp, datetime)) in timestamp_datetime.iter().enumerate() {
            let mut record = vec![timestamp.to_string(), datetime.to_string()];
            for channel in channels {
                let tempr: String = channel.data[i].1.map(|t| t.to_string()).unwrap_or_else(String::new);
                record.push(tempr)
            }
            writer.write_record(&record).unwrap();
        }
        
        writer.flush().unwrap();
        println!("Data saved to .CSV file");
    }
    
    // TODO: Make safe from panics. This is called from a different thread so the application will just keep going.
    fn read_input_from_serial(&self) {
        println!("Available serial ports: {:?}", self.port_names);

        loop {
            println!("self.selected_port_name: {:?}", self.selected_port_name);
            if self.selected_port_name.lock().unwrap().is_empty() {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            else{
                break;
            }
        }

        let port_name = self.selected_port_name.lock().unwrap().clone();

        let mut port = serialport::new(port_name, 9600)
            .timeout(std::time::Duration::from_secs(1))
            .open()
            .expect("Failed to open serial port");

        let mut serial_buf = String::new();

        loop {
            let mut buf: Vec<u8> = vec![0; 100];
            match port.read(buf.as_mut_slice()) {
                Ok(t) => {
                    if t == 0 {
                        continue;
                    }
                    let s = String::from_utf8_lossy(&buf[..t]);
                    serial_buf.push_str(&s);
                    while let Some(pos) = serial_buf.find("\r") {
                        let line = serial_buf[..pos].to_string();
                        self.append_data(&line);
                        serial_buf = serial_buf[pos + 1..].to_string();
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("Error reading from serial port: {}", e),
            }
        }

    }

    fn append_data (&self, new_data: &str) {
        // first check if str is an info string
        let first_char = new_data.chars().next().unwrap();
        if ['#', '?', '/', '-'].contains(&first_char) {
            self.handle_info_string(new_data);
            return;
        }

        // split the incoming data by commas
        let mut split_data = new_data.split(',');

        let time = split_data.next().unwrap().parse::<u64>().unwrap();
        let datetime_received = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        let mut channels = self.channels.lock().unwrap();
        for (channel, data_str) in channels.iter_mut().zip(split_data.take(NUM_CHANNELS)) {
            if data_str.is_empty() {
                channel.data.push((time, None));
                continue;
            }
            // convert the data to f64 while removing the last character (which is C for celsius)
            let value = data_str.trim_end_matches('C').parse::<f64>().unwrap();
            channel.data.push((time, Some(value)));
        }

        let mut timestamp_to_datetime = self.timestamp_datetime.lock().unwrap();
        timestamp_to_datetime.push((time, datetime_received));
    }


    fn handle_info_string(&self, info_string: &str) {
        // temporary print statement for testing
        println!("Info string received: {}", info_string);
    }
}


impl eframe::App for ThermometerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint(); // Request regular updates for real-time changes
        
        // Create the UI
        egui::CentralPanel::default().show(ctx, |ui| {

        
            let selected_port_name = self.selected_port_name.lock().unwrap().clone();
            egui::ComboBox::from_label("Select the serial port to read data from")
                .selected_text(&selected_port_name)
                .show_ui(ui, |ui| {
                    for port in &self.port_names {
                        if ui.selectable_label(selected_port_name == *port, port).clicked() {
                            *self.selected_port_name.lock().unwrap() = port.clone();
                        }
                    }
                });
    
            ui.heading("Current Temperature Data");
            
            //get window size
            let window_size = ui.available_size_before_wrap();

            let channels: &mut [Channel; NUM_CHANNELS] = &mut self.channels.lock().unwrap();
            // plot grid of values (2x4)
            egui::Grid::new("current_data_grid").show(ui, |ui| {
                const NUM_COLS: usize = 4;
                for (index, Channel{ data, enabled, colour }) in channels.iter_mut().enumerate() {
                    let temp = data.iter().last().map(|(_time, temp)| temp);
                    ui.group(|ui| {
                        ui.set_width(window_size.x * 0.25 - 6.0 * 3.0);

                        ui.horizontal(|ui| {
                            ui.label(format!("Sensor {}: ", index+1));
                            match temp {
                                Some(Some(f)) => ui.label(egui::RichText::new(format!("{:6.3}°C", f)).strong()),
                                _ => ui.label("No data"),
                            }
                        });    
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(enabled, "");
                            ui.color_edit_button_srgba(colour);
                        });
                    });
                    if (index+1) % NUM_COLS == 0 {
                        ui.end_row();
                    }
                }
            });
            
            // Save data to .CSV file on button press
            if ui.button("Save Data").on_hover_text("Save the current data to a .CSV file (YMD HMS for alphabetical sorting)").clicked() {
                let save_thread = self.clone();
                thread::spawn(move || {
                    save_thread.save_to_csv();
                });
            }

            ui.separator();

            ui.heading("Temperature Data Plot");

            let plot = egui_plot::Plot::new("data_plot");
            plot.show(ui, |plot_ui| {
                for Channel{ enabled, colour, data} in channels.iter() {
                    if *enabled {
                        // Filter out times with `None` temps
                        let all_points: Vec<[f64;2]> = data.iter().filter_map(|&(time, opt_temp)| opt_temp.map(|t| [time as f64, t])).collect();

                        // If we have many values only plot some of them
                        let points_per_pixel = (all_points.len() as f32 / window_size.x+0.01).round() as usize;
                        let plotted_points = if points_per_pixel < 2 { all_points }
                        else { all_points.into_iter().step_by(points_per_pixel).collect() };

                        plot_ui.line(egui_plot::Line::new(egui_plot::PlotPoints::from(plotted_points)).color(*colour));
                    }
                }
            });
        });
    }
}

fn main() {
    // Default line colours for the plot
    const DEFAULT_LINE_COLOURS: [eframe::egui::Color32; 8] = [
        egui::Color32::RED,
        egui::Color32::GREEN,
        egui::Color32::BLUE,
        egui::Color32::YELLOW,
        egui::Color32::from_rgba_premultiplied(255, 0, 255, 255), // magenta
        egui::Color32::from_rgba_premultiplied(0, 255, 255, 255), // cyan
        egui::Color32::WHITE,
        egui::Color32::GRAY
    ];

    const NUM_EXAMPLES: usize = 100_000;
    let channels: [Channel; NUM_CHANNELS] = std::array::from_fn(|i| Channel{ 
        // Some dummy data for testing lots of points
        data: std::array::from_fn::<_, NUM_EXAMPLES,_>( |j| 
            (j as u64, Some(f64::sin(j as f64 / 3000.0 + (i*20) as f64) // Nice sine wave example, each channel offset by 20 radians
         ))).to_vec(), 
        enabled: true, 
        colour: DEFAULT_LINE_COLOURS[i] }); 

    let timestamp_examples = std::iter::repeat_n((0, String::from("ABCD")), NUM_EXAMPLES).collect();

    let ports = serialport::available_ports().unwrap();
    let mut port_names = Vec::new();
    for port in ports {
        port_names.push(port.port_name);
    }

    let app = ThermometerApp {
        channels: Arc::new(Mutex::new(channels)),
        timestamp_datetime: Arc::new(Mutex::new(timestamp_examples)),
        port_names,
        selected_port_name: Arc::new(Mutex::new("".to_string())),
    };

    // thread to add data
    let app_read_in = app.clone();
    thread::spawn(move || {
        // app_read_in.read_input_from_cmd();
        app_read_in.read_input_from_serial();
    });

    // thread to autosave data
    let app_autosave = app.clone();
    thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(AUTOSAVE_SECONDS_INTERVAL));
            println!("Autosaving data...");
            app_autosave.save_to_csv();
        }
    });

    // Launch the application window
    eframe::run_native(
        "Thermometer Data",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|_cc|{
            Ok(Box::<ThermometerApp>::from(app))
        })).unwrap();
}