use eframe::{egui, epi};
use std::{io, sync::{Arc, Mutex}, thread};

const NUM_CHANNELS: usize = 8;
const AUTOSAVE_SECONDS_INTERVAL: u64 = 60;

#[derive(Clone)]
struct SerialDataPoint{
    time: u64, // time since thermometer start, in ms
    temperature: Vec<f32>, // temperature of each sensorm, stored in vector 
    time_received: String, //datetime of data received
}

#[derive(Clone)]
struct ThermometerApp{
    data : Arc<Mutex<Vec<SerialDataPoint>>>, // data from the serial port
    checked: Arc<Mutex<Vec<bool>>>, // whether the data for each channel is plotted
    colours: Arc<Mutex<Vec<[f32; 3]>>>, // line colours for each channel
}

impl ThermometerApp {
    fn save_to_csv(&self) {
        // write the data to a .csv file
        let current_time = chrono::Local::now().format("%Y-%m-%d %H-%M-%S").to_string();

        let mut writer = csv::Writer::from_path(format!("data {}.csv", current_time)).unwrap();

        let sensor_headers: Vec<String> = (1..=NUM_CHANNELS).map(|i| format!("Sensor {}", i)).collect();

        let mut headers = vec!["datetime of data", "Time since start (ms)"];
        headers.extend(sensor_headers.iter().map(|s| s.as_str()));
        writer.write_record(&headers).unwrap();

        let data = self.data.lock().unwrap();
        
        for data_point in data.iter() {
            let mut record = vec![data_point.time_received.clone()];
            record.push(data_point.time.to_string());
            for temp in &data_point.temperature {
                record.push(temp.to_string());
            }
            writer.write_record(&record).unwrap();
        }
        writer.flush().unwrap();
        println!("Data saved to .CSV file");
    }

    fn read_input_from_cmd(&self) {
        println!("Please enter new data (comma separated): ");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Input should be read in from the command line");
        let input = input.trim();
        self.append_data(input);
    }

    fn append_data (&self, new_data: &str) {
        // first check if str is an info string
        let first_char = new_data.chars().next().unwrap();
        if ['#', '?', '/', '-'].contains(&first_char) {
            self.handle_info_string(new_data);
            return;
        }

        // split the incoming data by commas
        let split_data: Vec<&str> = new_data.split(',').collect();

        let time = split_data[0].parse::<u64>().unwrap();
        let datetime_received = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        let mut temperatures = Vec::new();

        for &data_str in split_data.iter().skip(1).take(8) {
            if data_str.is_empty() {
                temperatures.push(f32::NAN);
                continue;
            }
            // convert the data to f32 while removing the last character (which is C for celsius)
            let value = data_str.trim_end_matches('C').parse::<f32>().unwrap();
            temperatures.push(value);
        }

        let new_data_point = SerialDataPoint {
            time,
            temperature: temperatures,
            time_received: datetime_received,
        };

        let mut data = self.data.lock().unwrap().clone();
        data.push(new_data_point);
    }


    fn handle_info_string(&self, info_string: &str) {
        // temporary print statement for testing
        println!("Info string received: {}", info_string);
    }
}


impl epi::App for ThermometerApp {

    fn name(&self) -> &str {
        "Thermometer Data"
    }
    

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        ctx.request_repaint(); // Request regular updates for real-time changes

        

        // Create the UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Current Temperature Data");

            //get window size
            let window_size = ui.available_size_before_wrap();

            let data_points = self.data.lock().unwrap().clone();

            // plot grid of values (2x4)
            egui::Grid::new("current_data_grid").show(ui, |ui| {
                let latest_data_point = data_points.last().unwrap();
                for row in 0..2 {
                    for col in 0..4 {

                        let index = row * 4 + col;
                        let current_sensor_temp = latest_data_point.temperature.get(index).unwrap();
                        let mut checked = self.checked.lock().unwrap();
                        let mut colours = self.colours.lock().unwrap();

                        ui.group(|ui| {
                            ui.set_width(window_size.x * 0.25 - 6.0 * 3.0);

                            ui.horizontal(|ui| {
                                ui.label(format!("Sensor {}: ", index+1));
                                if current_sensor_temp.is_nan() {
                                    ui.label("No data");
                                } else {
                                    ui.label(egui::RichText::new(format!("{:6.3}°C", current_sensor_temp)).strong());
                                }
                            });    
                            
                            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                ui.checkbox(&mut checked[index], "");
                                ui.color_edit_button_rgb(&mut colours[index]);
                            });
                        });

                    }
                    ui.end_row();
                }
            });
            
            // Save data to .CSV file on button press
            if ui.button("Save Data").on_hover_text("Save the current data to a .CSV file (YMD HMS for alphabetical sorting)").clicked() {
                self.save_to_csv();
            }

            ui.separator();

            ui.heading("Temperature Data Plot");


            let plot = egui::plot::Plot::new("data_plot");
            plot.show(ui, |plot_ui| {
                let checked = self.checked.lock().unwrap();
                let colours = self.colours.lock().unwrap();
                
                for i in 0..8 {
                    if checked[i] {
                        let color = egui::Color32::from_rgb(
                            (255.0 * colours[i][0]) as u8,
                            (255.0 * colours[i][1]) as u8,
                            (255.0 * colours[i][2]) as u8,
                        );
                        let times = data_points.iter().map(|d| d.time as f64).collect::<Vec<f64>>();
                        let temperatures = data_points.iter().map(|d| d.temperature[i] as f64).collect::<Vec<f64>>();
                        let values: Vec<egui::plot::Value> = times.iter().zip(temperatures.iter()).map(|(&t, &temp)| egui::plot::Value::new(t, temp)).collect();

                        plot_ui.line(egui::plot::Line::new(egui::plot::Values::from_values(values)).color(color));
                    }
                }
            });
        });
    }
}

fn main() {

    // Default line colours for the plot
    let default_line_colours = vec![
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
    ];

    
    let dummy_data_points = vec![
        SerialDataPoint {
            time: 0,
            temperature: vec![15.0, 15.4, 14.9, 15.2, 15.5, 15.7, 15.6, 15.78],
            time_received: "2024-12-11 12:00:00.000".to_string(),
        },
        SerialDataPoint {
            time: 125,
            temperature: vec![16.0, 16.1, 15.96, 16.13, 16.04 , 15.98, 16.02, 16.1],
            time_received: "2024-12-11 12:00:00.125".to_string(),
        },
        ];
    
    let app = ThermometerApp {
        data: Arc::new(Mutex::new(dummy_data_points)),
        checked: Arc::new(Mutex::new(vec![true; NUM_CHANNELS])),
        colours: Arc::new(Mutex::new(default_line_colours)),
    };

    // thread to add data
    let app_read_in = app.clone();
    thread::spawn(move || {
        loop {
            app_read_in.read_input_from_cmd();
        }
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
        Box::new(app),
        eframe::NativeOptions {
            maximized: true,
            ..Default::default()
        },
    );
}