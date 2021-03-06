#![feature(unboxed_closures)]
#![windows_subsystem = "windows"]

#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use cogset::{BruteScan, Dbscan};
use gtk::Orientation::{Horizontal, Vertical};
use gtk::{
    AboutDialogExt, BoxExt, CellLayoutExt, ContainerExt, DialogExt, FileChooserDialog,
    FileChooserExt, FileFilterExt, GtkWindowExt, Inhibit, LabelExt, Menu, MenuBar, MenuItem,
    MenuItemExt, MenuShellExt, OrientableExt, ScrolledWindowExt, TreeStoreExt, TreeStoreExtManual,
    TreeView, TreeViewColumnExt, TreeViewExt, Viewport, WidgetExt,
};
use location_history::{Locations, LocationsExt};
use osmgpsmap::{Map, MapExt, MapImage, MapOsd};
use relm::{Relm, Update, Widget};
use relm_attributes::widget;
use rexiv2::Metadata;
use rgeo::search;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use walkdir::WalkDir;

mod photo;

use self::MapMsg::*;
use self::MenuMsg::*;
use self::Msg::*;
use self::ViewMsg::*;
use crate::photo::{Photo, TimePhoto};

// The messages that can be sent to the update function.
#[derive(Msg)]
pub enum MenuMsg {
    SelectFile,
    SelectFolder,
    MenuAbout,
    MenuQuit,
}

#[derive(Clone)]
pub struct MyMenuBar {
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
        let menu_help = Menu::new();
        let menu_bar = MenuBar::new();

        let file = MenuItem::new_with_label("File");
        let quit = MenuItem::new_with_label("Quit");
        let folder_item = MenuItem::new_with_label("Import photos");
        let file_item = MenuItem::new_with_label("Import LocationHistory");

        let help = MenuItem::new_with_label("Help");
        let about = MenuItem::new_with_label("About");

        connect!(relm, quit, connect_activate(_), MenuQuit);
        connect!(relm, folder_item, connect_activate(_), SelectFolder);
        connect!(relm, file_item, connect_activate(_), SelectFile);
        connect!(relm, about, connect_activate(_), MenuAbout);

        menu_file.append(&folder_item);
        menu_file.append(&file_item);
        menu_file.append(&quit);
        file.set_submenu(Some(&menu_file));

        menu_help.append(&about);
        help.set_submenu(&menu_help);

        menu_bar.append(&file);
        menu_bar.append(&help);
        menu_bar.show_all();

        MyMenuBar { bar: menu_bar }
    }
}

#[derive(Clone)]
pub struct MyViewPort {
    view: Viewport,
    tree: TreeView,
}

#[derive(Msg)]
pub enum ViewMsg {
    UpdateView(gtk::TreeStore),
}

impl Update for MyViewPort {
    type Model = ();
    type ModelParam = ();
    type Msg = ViewMsg;

    fn model(_: &Relm<Self>, _: ()) {}

    fn update(&mut self, event: ViewMsg) {
        match event {
            UpdateView(model) => {
                self.tree.set_model(&model);
            }
        }
    }
}

impl Widget for MyViewPort {
    type Root = Viewport;

    fn root(&self) -> Self::Root {
        self.view.clone()
    }

    fn view(_relm: &Relm<Self>, _model: Self::Model) -> Self {
        // TODO: change column names and labels
        let view = Viewport::new(None, None);
        let tree = TreeView::new();
        let name_column = gtk::TreeViewColumn::new();
        let name_column_cell = gtk::CellRendererText::new();
        name_column.set_title("Name");
        name_column.pack_start(&name_column_cell, true);

        let start_column = gtk::TreeViewColumn::new();
        let start_column_cell = gtk::CellRendererText::new();
        start_column.set_title("Start date");
        start_column.pack_start(&start_column_cell, true);

        let end_column = gtk::TreeViewColumn::new();
        let end_column_cell = gtk::CellRendererText::new();
        end_column.set_title("End date");
        end_column.pack_start(&end_column_cell, true);

        tree.append_column(&name_column);
        //tree.append_column(&start_column);
        //tree.append_column(&end_column);

        name_column.add_attribute(&name_column_cell, "text", 0);
        start_column.add_attribute(&start_column_cell, "text", 1);
        end_column.add_attribute(&end_column_cell, "text", 2);

        view.add(&tree);

        view.show_all();

        MyViewPort { view, tree }
    }
}

#[derive(Msg)]
pub enum MapMsg {
    MarkLocation(f32, f32),
    ClearTags,
}

#[derive(Clone)]
pub struct MapModel {
    tags: Vec<MapImage>,
}

#[derive(Clone)]
pub struct MyMap {
    model: MapModel,
    hbox: gtk::Box,
    map: Map,
}

impl Update for MyMap {
    type Model = MapModel;
    type ModelParam = ();
    type Msg = MapMsg;

    fn model(_: &Relm<Self>, _: ()) -> MapModel {
        MapModel { tags: Vec::new() }
    }

    fn update(&mut self, event: MapMsg) {
        match event {
            MarkLocation(lat, long) => {
                // TODO: check if this can just be loaded once and reused
                let pointer =
                    gdk_pixbuf::Pixbuf::new_from_file_at_size("src/resources/pointer.svg", 80, 80)
                        .unwrap();
                // TODO: add these to a vector or something to track them
                if let Some(image) = self.map.image_add(lat, long, &pointer) {
                    self.model.tags.push(image);
                }
            }
            ClearTags => {
                while let Some(tag) = self.model.tags.pop() {
                    self.map.image_remove(&tag);
                }
            }
        }
    }
}

impl Widget for MyMap {
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.hbox.clone()
    }

    fn view(_relm: &Relm<Self>, model: Self::Model) -> Self {
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let map = Map::new();
        let osd = MapOsd::new();
        map.layer_add(&osd);
        hbox.pack_start(&map, true, true, 0);
        hbox.show_all();
        MyMap { hbox, map, model }
    }
}

//#[derive(Clone)]
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

#[widget]
impl Widget for Win {
    // The initial model.
    fn model() -> Model {
        Model {
            locations: Vec::new(),
            photos: Vec::new(),
        }
    }

    // Update the model according to the message received.
    fn update(&mut self, event: Msg) {
        match event {
            JsonDialog => {
                if let Some(x) = self.json_dialog() {
                    self.model.locations = self.load_json(x);
                    self.update_locations();
                };
            }
            FolderDialog => {
                if let Some(x) = self.folder_dialog() {
                    self.model.photos = self.load_photos(x);
                    self.update_locations();
                    self.view.emit(UpdateView(self.cluster_location()));
                    self.cluster_time();
                };
            }
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
                    MenuAbout => AboutDialog,
                    MenuQuit => Quit,
                },
                gtk::Box {
                    orientation: Horizontal,
                    #[name="map"]
                    MyMap {
                        property_expand: true,
                    },
                    gtk::Box {
                        orientation: Vertical,
                        gtk::Label {
                            text: "Clusters",
                        },
                        gtk::ScrolledWindow {
                            property_hscrollbar_policy: gtk::PolicyType::Never,
                            #[name="view"]
                            MyViewPort {
                                vexpand: true,
                                property_width_request: 300,
                            },
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
        let mut path = None;
        let dialog = FileChooserDialog::new::<gtk::Window>(
            Some("Import File"),
            Some(&self.root()),
            gtk::FileChooserAction::SelectFolder,
        );
        dialog.add_button("Ok", gtk::ResponseType::Ok.into());
        dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
        let response_ok: i32 = gtk::ResponseType::Ok.into();
        if dialog.run() == response_ok {
            path = dialog.get_filename();
        }
        dialog.destroy();
        path
    }

    fn about_dialog(&self) {
        let dialog = gtk::AboutDialog::new();
        dialog.set_transient_for(&self.root());
        dialog.set_modal(true);
        dialog.set_authors(&["Eric Trombly"]);
        dialog.set_program_name("Photos-rs");
        dialog.set_comments("Photo tagger");
        if let Ok(logo) = gdk_pixbuf::Pixbuf::new_from_file("resources/Antu_map-globe.ico") {
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
        let files = WalkDir::new(path).into_iter().filter_map(|e| e.ok());
        let files = files.filter(|x| Metadata::new_from_path(x.path()).is_ok());
        files.map(|x| Photo::new(x.path().to_path_buf())).collect()
    }

    fn update_locations(&mut self) {
        for photo in &mut self.model.photos {
            if photo.location == None {
                if let Some(time) = photo.time {
                    if let Some(closest) = self.model.locations.find_closest(time) {
                        photo.set_location(closest);
                    }
                }
            }
        }
    }

    fn cluster_location(&self) -> gtk::TreeStore {
        self.map.emit(ClearTags);
        let scanner = BruteScan::new(&self.model.photos);
        let mut dbscan = Dbscan::new(scanner, 1000.0, 3);
        let clusters = dbscan.by_ref().collect::<Vec<_>>();
        let model = gtk::TreeStore::new(&[gtk::Type::String, gtk::Type::String, gtk::Type::String]);
        for cluster in clusters {
            let top = model.append(None);
            if let Some(x) = cluster
                .iter()
                .find(|&&x| self.model.photos[x].location_name.is_some())
            {
                model.set(
                    &top,
                    &[0],
                    &[self.model.photos[*x].location_name.as_ref().unwrap()],
                );
            } else if let Some(point) = self.model.photos[cluster[0]].location {
                if let Some(loc) = search(point.y(), point.x()) {
                    model.set(&top, &[0], &[&format!("{}, {}", loc.1.name, loc.1.country)]);
                    self.map.emit(MarkLocation(point.y(), point.x()));
                }
            }
            for photo in cluster {
                let entries = model.append(&top);
                model.set(
                    &entries,
                    &[0],
                    &[&format!("{:?} ", self.model.photos[photo].path)],
                );
            }
        }
        model
    }

    fn cluster_time(&self) {
        let timephotos = self
            .model
            .photos
            .iter()
            .map(|x| TimePhoto(x))
            .collect::<Vec<_>>();
        let timescanner = BruteScan::new(&timephotos);
        let mut timedbscan = Dbscan::new(timescanner, 600.0, 10);
        let _timeclusters = timedbscan.by_ref().collect::<Vec<_>>();
    }
}

fn main() {
    Win::run(()).unwrap();
}
