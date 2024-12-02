use eframe::{egui, epi};
use chrono;
use std::{sync::{Arc, Mutex}, vec, thread};
use std::io::{self};

struct SerialInputData {
    data: Arc<Mutex<Vec<Vec<f32>>>>,  // Stores the incoming serial data
    time : Arc<Mutex<Vec<f32>>>, // Stores the time data
    checked: Vec<bool>, // Stores the checked state of the checkboxes
}

impl SerialInputData {
    fn save_to_csv(&self) {
        // Save the data to a .CSV file
        println!("Data saved to .CSV file");

        let data = self.data.lock().unwrap();
        let time = self.time.lock().unwrap();
        let current_time = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let mut writer = csv::Writer::from_path(format!("data{}.csv", current_time)).unwrap();
        writer.write_record(&["Time", "Sensor 1", "Sensor 2", "Sensor 3", "Sensor 4", "Sensor 5", "Sensor 6", "Sensor 7", "Sensor 8"]).unwrap();
        for i in 0..time.len() {
            let mut record = vec![time[i].to_string()];
            for j in 0..8 {
                record.push(data[j][i].to_string());
            }
            writer.write_record(&record).unwrap();
        }
        writer.flush().unwrap();   
    }

    fn append_data (&self, new_data: &str) {
        // split the incoming data by commas
        let split_data: Vec<&str> = new_data.split(',').collect();
        self.time.lock().unwrap().push(split_data[0].parse::<f32>().unwrap());
        for i in 1..9 {
            // convert the data to f32 while removing the last character (which is C for celsius) and skip the first element (which is the time)
            let value = split_data[i].trim_end_matches('C').parse::<f32>().unwrap();
            self.data.lock().unwrap()[i-1].push(value);
        }
    }

    fn read_input_from_cmd(&self) {
        println!("Please enter new data (comma separated): ");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let input = input.trim();
        self.append_data(input);
    }
}

impl epi::App for SerialInputData {

    fn name(&self) -> &str {
        "Thermometer Data"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        ctx.request_repaint(); // Request regular updates for real-time changes

        let data = self.data.lock().unwrap().clone();


        

        // GUI drawing logic (plotting the serial data)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Current Temperature Data");

            // 2 by 4 grid of current data

            //get window size
            let window_size = ui.available_size_before_wrap();

        
            
            egui::Grid::new("current_data_grid").show(ui, |ui| {
                let spacing = ui.spacing_mut();
                spacing.item_spacing = egui::vec2(0.0, 0.0); // Add some spacing between the elements
                for i in 0..2 {
                    for j in 0..4 {
                        let index = i * 4 + j;
                        let value = data.get(index).and_then(|inner| inner.last()).unwrap_or(&0.0);
                        ui.group(|ui| {
                            ui.set_width(window_size.x * 0.25 - 6.0 * 3.0);
                            ui.horizontal(|ui| {
                                ui.label(format!("Sensor {}: ", index));
                                ui.label(egui::RichText::new(format!("{:6.3}°C", value)).strong());
                            });    
                            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                ui.checkbox(&mut self.checked[index], "");
                            });
                        });
                    }
                    ui.end_row();
                }
            });
            

            if ui.button("Save Data").on_hover_text("Save the current data to a .CSV file").clicked() {
                self.save_to_csv();
            }

            ui.separator();

            ui.heading("Temperature Data Plot");


            let plot = egui::plot::Plot::new("data_plot");
            plot.show(ui, |plot_ui| {
                for i in 0..8 {
                    if self.checked[i] {
                        let color = egui::Color32::from_rgb(
                            (i * 32) as u8,
                            (255 - i * 32) as u8,
                            (i * 16) as u8,
                        );
                        let time = self.time.lock().unwrap();
                        let data: Vec<_> = data.get(i).unwrap().iter().enumerate().map(|(i, &v)| egui::plot::Value::new(time[i] as f64, v as f64)).collect();
                        plot_ui.line(egui::plot::Line::new(egui::plot::Values::from_values(data)).color(color));
                    }
                }
            });
        });
    }
}

fn main() {
    let dummy_data = vec![
        vec![15.0, 15.4, 14.9, 15.2, 15.5, 15.7, 15.6, 15.78],
        vec![16.0, 16.1, 15.96, 16.13, 16.04 , 15.98, 16.02, 16.1],
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],   
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ];

    let dummy_time = vec![0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 1.6];

    // Set up the app
    let app = SerialInputData {
        data: Arc::new(Mutex::new(dummy_data)),
        time: Arc::new(Mutex::new(dummy_time)),
        checked: vec![true; 8],
    };


    // Clone the Arc references before moving app into the thread
    let app_data = Arc::clone(&app.data);
    let app_time = Arc::clone(&app.time);
    let app_checked = app.checked.clone();

    // Spawn a new thread to run read_input_from_cmd
    thread::spawn(move || {
        let app = SerialInputData {
            data: app_data,
            time: app_time,
            checked: app_checked,
        };
        loop {
            app.read_input_from_cmd();
        }
    });

    // Launch the application window with `eframe`
    eframe::run_native(
        Box::new(app),
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(800.0, 600.0)),
            ..Default::default()
        },
    );
}