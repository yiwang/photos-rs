#![feature(proc_macro)]
#![windows_subsystem = "windows"]

extern crate chrono;
extern crate rexiv2;
extern crate location_history;
extern crate geo;
extern crate cogset;
extern crate walkdir;
extern crate gtk;
extern crate gdk_pixbuf;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;

use rexiv2::Metadata;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use location_history::{Locations, LocationsExt};
use cogset::{Dbscan, BruteScan};
use walkdir::WalkDir;
use gtk::{BoxExt, CellLayoutExt, ContainerExt, FileChooserDialog, FileChooserExt,
          DialogExt, Inhibit, Menu, MenuBar, MenuItem, MenuItemExt, MenuShellExt, OrientableExt,
          ScrolledWindowExt, TreeView, Viewport, WidgetExt, WindowExt};
use gtk::Orientation::{Vertical, Horizontal};
use relm::{Relm, Update, Widget};
use relm_attributes::widget;

mod photo;

use photo::{Photo, TimePhoto};
use self::Msg::*;
use self::ViewMsg::*;
use self::MenuMsg::*;

// The messages that can be sent to the update function.
#[derive(Msg)]
enum MenuMsg {
    SelectFile,
    SelectFolder,
    SortOrder(SortBy),
    MenuAbout,
    MenuQuit,
}

#[derive(Clone)]
struct MyMenuBar {
    bar: MenuBar,
}

/// all the events are handled in Win
impl Update for MyMenuBar {
    type Model = ();
    type ModelParam = ();
    type Msg = MenuMsg;

    fn model(_: &Relm<Self>, _: ()) {}

    fn update(&mut self, _event: MenuMsg) {}
}

impl Widget for MyMenuBar {
    type Root = MenuBar;

    fn root(&self) -> Self::Root {
        self.bar.clone()
    }

    fn view(relm: &Relm<Self>, _model: Self::Model) -> Self {
        let menu_file = Menu::new();
        let menu_sort = Menu::new();
        let menu_help = Menu::new();
        let menu_bar = MenuBar::new();

        let file = MenuItem::new_with_label("File");
        let quit = MenuItem::new_with_label("Quit");
        let folder_item = MenuItem::new_with_label("Import photos");
        let file_item = MenuItem::new_with_label("Import LocationHistory");

        let sort = MenuItem::new_with_label("Sort");
        let year = MenuItem::new_with_label("Year");
        let country = MenuItem::new_with_label("Country");

        let help = MenuItem::new_with_label("Help");
        let about = MenuItem::new_with_label("About");

        connect!(relm, quit, connect_activate(_), MenuQuit);
        connect!(relm, folder_item, connect_activate(_), SelectFolder);
        connect!(relm, file_item, connect_activate(_), SelectFile);
        connect!(relm, year, connect_activate(_), SortOrder(SortBy::Year));
        connect!(relm, country, connect_activate(_), SortOrder(SortBy::Country));
        connect!(relm, about, connect_activate(_), MenuAbout);

        menu_file.append(&folder_item);
        menu_file.append(&file_item);
        menu_file.append(&quit);
        file.set_submenu(Some(&menu_file));

        menu_sort.append(&year);
        menu_sort.append(&country);
        sort.set_submenu(&menu_sort);

        menu_help.append(&about);
        help.set_submenu(&menu_help);

        menu_bar.append(&file);
        menu_bar.append(&sort);
        menu_bar.append(&help);
        menu_bar.show_all();

        MyMenuBar { bar: menu_bar }
    }
}

#[derive(Clone)]
struct MyViewPort {
    model: ViewModel,
    view: Viewport,
    tree: TreeView,
}

#[derive(Clone)]
pub struct ViewModel {
    order: SortBy,
}

#[derive(Msg)]
pub enum ViewMsg {
    SortChanged(SortBy),
}

impl Update for MyViewPort {
    type Model = ViewModel;
    type ModelParam = ();
    type Msg = ViewMsg;

    fn model(_: &Relm<Self>, _: ()) -> ViewModel {
        ViewModel {
            order: SortBy::Year,
        }
    }

    fn update(&mut self, event: ViewMsg) {
        match event {
            SortChanged(order) => {
                self.model.order = order;
                self.update_tree_model();
            }
        }
    }
}

impl Widget for MyViewPort {
    type Root = Viewport;

    fn root(&self) -> Self::Root {
        self.view.clone()
    }

    fn view(_relm: &Relm<Self>, model: Self::Model) -> Self {
        let view = Viewport::new(None, None);
        let tree = TreeView::new();
        let country_column = gtk::TreeViewColumn::new();
        let country_column_cell = gtk::CellRendererText::new();
        country_column.set_title("Country");
        country_column.pack_start(&country_column_cell, true);

        let start_column = gtk::TreeViewColumn::new();
        let start_column_cell = gtk::CellRendererText::new();
        start_column.set_title("Start date");
        start_column.pack_start(&start_column_cell, true);

        let end_column = gtk::TreeViewColumn::new();
        let end_column_cell = gtk::CellRendererText::new();
        end_column.set_title("End date");
        end_column.pack_start(&end_column_cell, true);

        tree.append_column(&country_column);
        tree.append_column(&start_column);
        tree.append_column(&end_column);

        country_column.add_attribute(&country_column_cell, "text", 0);
        start_column.add_attribute(&start_column_cell, "text", 1);
        end_column.add_attribute(&end_column_cell, "text", 2);

        view.add(&tree);

        view.show_all();

        MyViewPort { model, view, tree }
    }
}

impl MyViewPort {
    fn update_tree_model(&self) {
    }
}

#[derive(Clone)]
pub struct Model {
    locations: Locations,
    photos: Vec<Photo>,
}

#[derive(Msg)]
pub enum Msg {
    JsonDialog,
    FolderDialog,
    AboutDialog,
    Quit,
}

#[derive(Clone)]
pub enum SortBy {
    Country,
    Year,
}

#[widget]
impl Widget for Win {
    // The initial model.
    fn model() -> Model {
        Model {locations: Vec::new(), photos: Vec::new()}
    }

    // Update the model according to the message received.
    fn update(&mut self, event: Msg) {
        match event {
            JsonDialog => {
                if let Some(x) = self.json_dialog() {
                    self.model.locations = self.load_json(x);
                    self.update_locations();
                };
            },
            FolderDialog => {
                if let Some(x) = self.folder_dialog() {
                    self.model.photos = self.load_photos(x);
                    self.update_locations();
                    self.cluster_location();
                    self.cluster_time();
                };
            },
            AboutDialog => self.about_dialog(),
            Quit => gtk::main_quit(),
        }
    }

    view! {
        #[name="root"]
        gtk::Window {
            title: "Photos-rs",
            gtk::Box {
                // Set the orientation property of the Box.
                orientation: Vertical,
                MyMenuBar {
                    SelectFile => JsonDialog,
                    SelectFolder => FolderDialog,
                    SortOrder(ref x) => view@SortChanged(x.clone()),
                    MenuAbout => AboutDialog,
                    MenuQuit => Quit,
                },
                gtk::Box {
                    orientation: Horizontal,
                    gtk::DrawingArea {
                        packing: {
                                expand: true,
                        },
                    },
                    gtk::Box{
                        orientation: Vertical,
                        gtk::Label {
                            text: "Clusters",
                        },
                        gtk::ScrolledWindow {
                            property_hscrollbar_policy: gtk::PolicyType::Never,
                            #[name="view"]
                            MyViewPort,
                        },
                    },
                },
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

impl Win {
    fn json_dialog(&self) -> Option<PathBuf> {
        let dialog = FileChooserDialog::new::<gtk::Window>(
            Some("Import File"),
            Some(&self.root()),
            gtk::FileChooserAction::Open,
        );
        let filter = gtk::FileFilter::new();
        filter.set_name("json");
        filter.add_pattern("*.json");
        dialog.add_filter(&filter);
        dialog.add_button("Ok", gtk::ResponseType::Ok.into());
        dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
        let response_ok: i32 = gtk::ResponseType::Ok.into();
        if dialog.run() == response_ok {
            let path = dialog.get_filename();
            dialog.destroy();
            return path;
        }
        dialog.destroy();
        None
    }

    fn folder_dialog(&self) -> Option<PathBuf> {
        let dialog = FileChooserDialog::new::<gtk::Window>(
            Some("Import File"),
            Some(&self.root()),
            gtk::FileChooserAction::SelectFolder,
        );
        dialog.add_button("Ok", gtk::ResponseType::Ok.into());
        dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
        let response_ok: i32 = gtk::ResponseType::Ok.into();
        if dialog.run() == response_ok {
            let path = dialog.get_filename();
            dialog.destroy();
            return path;
        }
        dialog.destroy();
        None
    }

    fn about_dialog(&self) {
        let dialog = gtk::AboutDialog::new();
        dialog.set_transient_for(&self.root());
        dialog.set_modal(true);
        dialog.set_authors(&["Eric Trombly"]);
        dialog.set_program_name("Photos-rs");
        dialog.set_comments("Photo tagger");
        if let Ok(logo) = gdk_pixbuf::Pixbuf::new_from_file("Antu_map-globe.ico") {
            dialog.set_logo(Some(&logo));
        };
        dialog.run();
        dialog.destroy();
    }

    fn load_json(&self, path: PathBuf) -> Locations {
        // read json file
        let mut contents = String::new();
        File::open(path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        location_history::deserialize(&contents).filter_outliers()
    }

    fn load_photos(&self, path: PathBuf) -> Vec<Photo> {
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
        println!("Scanning photos");
        let files = WalkDir::new(path).into_iter().filter_map(|e| e.ok());
        let files = files.filter(|x| Metadata::new_from_path(x.path()).is_ok());
        files.map(|x| Photo::new(x.path().to_path_buf())).collect()
    }

    fn update_locations(&mut self) {
        for photo in self.model.photos.iter_mut() {
            if photo.location == None {
                if let Some(time) = photo.time {
                    println!("  Date: {:?}", time);
                    if let Some(closest) = self.model.locations.find_closest(time) {
                        println!(
                            "  closest timestamp: {:?} long: {} lat: {} accuracy: {}",
                            closest.timestamp,
                            closest.longitude,
                            closest.latitude,
                            closest.accuracy
                        );
                        photo.set_location(closest);
                    }
                }
            }
        }
    }

    fn cluster_location(&self) {
        let scanner = BruteScan::new(&self.model.photos);
        let mut dbscan = Dbscan::new(scanner, 1000.0, 3);
        let clusters = dbscan.by_ref().collect::<Vec<_>>();
        for cluster in clusters {
            println!("Cluster located near {:?}", self.model.photos[cluster[0]].location);
            for photo in cluster {
                print!("{:?} ", self.model.photos[photo].path);
            }
            println!("\n");
        }
    }

    fn cluster_time(&self) {
        let timephotos = self.model.photos.iter().map(|x| TimePhoto(x)).collect::<Vec<_>>();
        let timescanner = BruteScan::new(&timephotos);
        let mut timedbscan = Dbscan::new(timescanner, 600.0, 10);
        let timeclusters = timedbscan.by_ref().collect::<Vec<_>>();
        for cluster in timeclusters {
            println!("Cluster located at {:?}", timephotos[cluster[0]].0.time);
            for photo in cluster {
                print!("{:?} ", timephotos[photo].0.path);
            }
            println!("\n");
        }
    }
}

fn main() {
    Win::run(()).unwrap();
}