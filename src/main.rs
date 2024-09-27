//! Solitaire
use iced::widget::{button, column, container, row, vertical_space, text, pick_list};
use iced::Length::Fill;
use iced::{Element, Length, Padding, Size, Theme};

pub fn main() -> iced::Result {
    iced::application("Solitaire", Solitaire::update, Solitaire::view)
        .theme(|_| Theme::CatppuccinMocha)
        .antialiasing(true)
        .centered()
        .window_size(Size{ width: 1100.0, height: 800.0 })
        .run()
}

#[derive(Default)]
struct Solitaire {
    board: board::State,
}

#[derive(Debug, Clone)]
enum Message {
    MoveCard(board::CardPosition),
    Start,
    SelectCardsToPlay(String),
}

impl Solitaire {
    fn update(&mut self, message: Message) {
        match message {
            Message::MoveCard(positions) => {
                self.board.position = positions.clone();
                let areas = self.board.move_cards(positions);
                self.board.recalc_tab_positions();
                self.board.request_redraw(areas);
            },
            Message::Start => {
                self.board.start();
                for i in 0..7 {
                    self.board.tab_cache[i].clear();
                }
                self.board.waste_cache.clear();
                self.board.stock_cache.clear();
                self.board.foundation_cache.clear();
            },
            Message::SelectCardsToPlay(selected) => {
                self.board.cards_to_play = Some(selected);
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let mut start_label = "Play";
        if self.board.start {
            start_label = "Play Again";
        }
        let btn_start: Element<Message> = button(start_label).on_press(Message::Start).into();

        let rounds_str: Element<Message> = text("Card Play Rounds:").into();
        let rounds_value: Element<Message> = text(format!("{}", self.board.card_rounds))
                                            .width(Fill)
                                            .into();

        let moved_from_waste_str: Element<Message> = text("Moved from Waste: ").into();
        let moved_from_waste_value: Element<Message> = text(format!("{}", self.board.cards_moved_from_waste))
                                                        .width(Fill)
                                                        .into();

        let to_play_text: Element<Message> = text("Cards to Play").into();
        let cards_to_play: Element<Message> = pick_list(vec!["3".to_string(), "1".to_string()], 
                                                self.board.cards_to_play.clone(),
                                                Message::SelectCardsToPlay)
                                                .into();

        let instruction_space: Element<Message> = vertical_space().height(75.0).into();

        let instructions: Element<Message> = text("Instructions:\nCards are moved by selecting source and destination using mouse.  If a card fails to move it means the validation failed, wrong color or value.\nTo cancel a move, click any other place on the canvas").into();        
        
        let col: Element<Message> = column(vec![btn_start, 
                                                            rounds_str,
                                                            rounds_value, 
                                                            moved_from_waste_str,
                                                            moved_from_waste_value,
                                                            to_play_text,
                                                            cards_to_play,
                                                            instruction_space,
                                                            instructions,
                                                            ])
                                            .width(Length::Fixed(130.0))
                                            .spacing(10.0)
                                            .padding(Padding{ top: 20.0, right: 0.0, bottom: 0.0, left: 20.0 })
                                            .into();

        let cont = container(
            self.board.view().map(Message::MoveCard)
        )
        .padding(Padding{ top: 20.0, right: 20.0, bottom: 20.0, left: 0.0 })
        .into();

        row(vec![col, cont]).into()

    }
}

mod board {
    use iced::advanced::image::Handle;
    use iced::{mouse, Color};
    use iced::widget::canvas::event::{self, Event};
    use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Style};
    use iced::{Element, Fill, Point, Rectangle, Renderer, Theme};

    use rand::seq::SliceRandom;
    use rand::thread_rng;

    #[derive()]
    pub struct State {
        pub start: bool,

        pub foundation_cache: canvas::Cache,
        pub stock_cache: canvas::Cache,
        pub table_cache: canvas::Cache,
        pub waste_cache: canvas::Cache,
        pub tab_cache: Vec<canvas::Cache>,

        pub position: CardPosition,

        cover_image: Handle,
        cards: Vec<Card>,

        tab_card_indexes: Vec<Vec<usize>>,
        tab_positions: Vec<Vec<(Point, Point)>>,
        
        tab_x_offset_pos: f32,
        tab_y_offset_pos: f32,
        card_dist_x: f32,
        card_dist_y: f32,
        card_size_x: f32,
        card_size_y: f32,
        
        foundation_cards: Vec<usize>,
        foundation_positions: Vec<(Point, Point)>,
        
        stock_position: (Point, Point),
        stock_card_indexes: Vec<usize>,
        stock_area_image_index: Option<usize>,

        waste_position: (Point, Point),
        waste_card_indexes: Vec<usize>,
        waste_area_image_index: Option<usize>,

        pub card_rounds: u16,
        pub cards_moved_from_waste: u16,
        pub cards_to_play: Option<String>,
    }

    impl State {
        pub fn new() -> State {
            let path = format!("{}/assets/cards/card_back.png",
                env!("CARGO_MANIFEST_DIR"));

            let card_dist_x = 120.0;
            let card_dist_y = 25.0;
            
            // init the foundation positions
            let mut foundation_positions = vec![];
            for i in 0..4 {
                foundation_positions.push((Point { x: 400.0 + card_dist_x * i as f32, y: 25.0},
                                    Point { x: 400.0 + card_dist_x * i as f32 + 100.0, y: 175.0}));
            }

            let mut tab_cache = vec![];
            for _ in 0..7 {
                tab_cache.push(canvas::Cache::default());
            }

            State {
                start: false,

                foundation_cache: canvas::Cache::default(),
                table_cache: canvas::Cache::default(),
                stock_cache: canvas::Cache::default(),
                waste_cache: canvas::Cache::default(),
                tab_cache,
                position: CardPosition { from: Point::ORIGIN, to: Point::ORIGIN },
            
                cover_image: Handle::from_path(path),
                cards: vec![],
                
                tab_card_indexes: vec![],
                tab_positions: vec![],

                tab_x_offset_pos: 50.0,
                tab_y_offset_pos: 250.0,
                card_dist_x,
                card_dist_y,
                card_size_x: 100.0,
                card_size_y: 150.0,

                foundation_cards: vec![100, 100, 100, 100],
                foundation_positions,

                stock_card_indexes: vec![],
                stock_position: (Point{ x: 50.0, y: 25.0}, Point{ x: 150.0, y: 175.0 }),
                stock_area_image_index: Some(100),

                waste_position: (Point{ x: 170.0, y: 25.0}, Point{ x: 270.0, y: 175.0 }),
                waste_card_indexes: vec![],
                waste_area_image_index: None,

                card_rounds: 0,
                cards_moved_from_waste: 0,
                cards_to_play: Some("3".to_string()),
            }
        }

        pub fn view<'a>(&'a self) -> Element<'a, CardPosition> {
            Canvas::new(CardsDraw {
                state: self,
            })
            .width(Fill)
            .height(Fill)
            .into()
        }

        pub fn request_redraw(&mut self, areas: Vec<Area>) {
            for area in areas {
                match area {
                    Area::None => (),
                    Area::Foundation(_) => {
                        self.foundation_cache.clear()
                    },
                    Area::Stock => {
                        self.stock_cache.clear()
                    },
                    Area::Waste => {
                        self.waste_cache.clear()
                    },
                    Area::Tableau(tab) => {
                        self.tab_cache[tab].clear()
                    },
                }
            }
            
            
        }
    }

    
    impl State {
        pub fn start(&mut self) {

            self.cards = load_cards();
            let tableau = [1, 2, 3, 4, 5, 6, 7];
            self.tab_positions = vec![vec![]; 7];
            self.tab_card_indexes = vec![vec![]; 7];
            self.foundation_cards = vec![100; 4];
            self.stock_card_indexes = vec![];
            self.waste_card_indexes = vec![];
            self.waste_area_image_index = None;
            self.card_rounds = 0;
            self.cards_moved_from_waste = 0;

            let mut card_index = 0;
            for (i, tab_col) in tableau.iter().enumerate() {
                for j in 0..*tab_col {
                    if j == *tab_col-1 {
                        self.tab_card_indexes[i].push(card_index);
                        self.tab_positions[i].push((Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32, 
                                                            y: self.tab_y_offset_pos + self.card_dist_y * j as f32},
                                                    Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32 + self.card_size_x, 
                                                            y: self.tab_y_offset_pos + self.card_dist_y + self.card_dist_y * j as f32 + self.card_size_y}));
                        self.cards[card_index].visible = true;
                    } else {
                        self.tab_card_indexes[i].push(card_index);
                        self.tab_positions[i].push((Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32,
                                                            y: self.tab_y_offset_pos + self.card_dist_y * j as f32},
                                                    Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32 + self.card_size_x, 
                                                            y: self.tab_y_offset_pos + self.card_dist_y + self.card_dist_y * j as f32}));
                    }
                    card_index += 1;
                }  
    }
            // add cards left to stock
            for i in card_index..self.cards.len() {
                self.cards[card_index].visible = true;
                self.stock_card_indexes.push(i);
            }
            self.stock_area_image_index = Some(100);
            self.start = true;
   
        }

        pub fn move_cards(&mut self, positions: CardPosition) -> Vec<Area> {
    
            // Check stock to waste area
            if point_in_area(positions.from, self.stock_position) 
                && point_in_area(positions.to, self.waste_position){
                self.move_stock_to_waste();
                return vec![Area::Stock, Area::Waste]
            }

            // Check waste to stock area
            if point_in_area(positions.from, self.waste_position) 
                && point_in_area(positions.to, self.stock_position){
                self.move_waste_to_stock();
                return vec![Area::Waste, Area::Stock]
            }

            // check if waste to tab
            let waste_area = point_in_area(positions.from, self.waste_position);
            let (tab_index_to, _tab_card_index) = self.find_tab_area(positions.to);
            // waste to tab only uses the tab index not the index of the tab column
            if tab_index_to.is_some() && waste_area {
                self.move_waste_to_tab(tab_index_to.unwrap());
                return vec![Area::Waste, Area::Tableau(tab_index_to.unwrap())]
            }

            let fd_index_to = self.find_foundation_area(positions.to);
            // check if waste to fd
            if fd_index_to.is_some() && waste_area {
                self.move_waste_to_foundation(fd_index_to.unwrap());
                return vec![Area::Waste, Area::Foundation(fd_index_to.unwrap())]
            }

            // check if tab
            let (tab_index_from_opt, tab_card_index_from_opt) = self.find_tab_area(positions.from);
            
            // check if tab to tab
            let (tab_index_to_opt, tab_card_index_to_opt) = self.find_tab_area(positions.to);
            if tab_index_from_opt.is_some() && tab_index_to_opt.is_some() {
                self.move_tab_to_tab((tab_index_from_opt.unwrap(), tab_card_index_from_opt.unwrap()), 
                                    (tab_index_to_opt.unwrap(), tab_card_index_to_opt.unwrap()));
                return vec![Area::Tableau(tab_index_from_opt.unwrap()), Area::Tableau(tab_index_to.unwrap())]
            }
            
            // check if tab to fd
            if tab_index_from_opt.is_some() && fd_index_to.is_some()  {
                self.move_tab_to_foundation(tab_index_from_opt.unwrap(), fd_index_to.unwrap());
                return vec![Area::Tableau(tab_index_from_opt.unwrap()), Area::Foundation(fd_index_to.unwrap())]
            }

            vec![Area::None]

        }

        pub fn find_tab_area(&mut self, position: Point) -> (Option<usize>, Option<usize>) {

            for i in 0..7 {
                if self.tab_positions[i].len() == 0 {
                    continue;
                }
                // postion of first card index
                let first_card = self.tab_positions[i][0].0;

                let last_index = if self.tab_positions[i].len() > 0 {
                    self.tab_positions[i].len()-1
                } else {
                    0
                };
                
                let last_card = self.tab_positions[i][last_index].1;
                if point_in_area(position, (first_card, last_card)) {
                    for (index, area) in self.tab_positions[i].iter().enumerate() {
                        if point_in_area(position, *area) {
                            return (Some(i), Some(index));
                        }
                    } 
                }
            }
 
            return (None, None)
        }

        pub fn find_foundation_area(&mut self, position: Point) -> Option<usize> {

            for (i, fd_pos) in self.foundation_positions.iter().enumerate() {
                let found_to = point_in_area(position, fd_pos.clone());
                if found_to {
                    return Some(i);
                }
            }
            
            None
        }

        pub fn move_stock_to_waste(&mut self) {
            if self.stock_card_indexes.len() == 0 {
                return
            }
            if self.cards_to_play == Some("1".to_string()) {
                let final_length = self.stock_card_indexes.len().saturating_sub(1);
                let tail = self.stock_card_indexes.split_off(final_length);
                self.waste_card_indexes.extend(tail);
            } else {
                if self.stock_card_indexes.len() >= 3 {
                        let final_length = self.stock_card_indexes.len().saturating_sub(3);
                        let mut tail = self.stock_card_indexes.split_off(final_length);
                        tail.reverse();
                        self.waste_card_indexes.extend(tail);
                } else if self.stock_card_indexes.len() <= 3 {
                    self.waste_card_indexes.extend(self.stock_card_indexes.clone());
                    self.stock_card_indexes = vec![];
                    self.stock_area_image_index = None;
                }
            }

            if self.stock_card_indexes.len() == 0 {
                self.stock_area_image_index = None;
            } else {
                self.stock_area_image_index = Some(100);
            }

            // show top card
            self.waste_area_image_index = if self.waste_card_indexes.len() > 0 {
                Some(*self.waste_card_indexes.last().unwrap())
            } else {
                None
            };
        }

        // move cards back only if stock is empty
        pub fn move_waste_to_stock(&mut self) {
            if self.stock_card_indexes.is_empty() {
                if self.waste_card_indexes.is_empty() {
                    return
                }
                self.waste_card_indexes.reverse();
                self.stock_card_indexes = self.waste_card_indexes.clone();
                self.waste_card_indexes = vec![];
                self.stock_area_image_index = Some(100);
                self.waste_area_image_index = None;
                self.card_rounds += 1;
                self.cards_moved_from_waste = 0;
            }
        }

        pub fn move_waste_to_tab(&mut self, tab_index: usize) {
            let waste_card_index_opt = self.waste_card_indexes.last();
            let last_tab_card_index_opt = self.tab_card_indexes[tab_index].last();

            let last_tab_card_index_opt = match last_tab_card_index_opt {
                Some(index) => Some(*index),
                None => None,
            };

            let king_card: Option<usize> = if waste_card_index_opt.is_some() {
                if self.cards[*waste_card_index_opt.unwrap()].value == 13 {
                    Some(13)
                }
                else {
                    None
                }
            } else {
                None
            };

            if waste_card_index_opt.is_some() && (last_tab_card_index_opt.is_some() || king_card.is_some()) {
                // if tab is empty then a king can be put in
                if waste_card_index_opt.is_some() && last_tab_card_index_opt.is_none() {
                    let waste_card_index = waste_card_index_opt.unwrap();
                    if king_card.is_some() {
                        self.cards[*waste_card_index].visible = true;
                        self.tab_card_indexes[tab_index].push(*waste_card_index);
                        let final_length = self.waste_card_indexes.len().saturating_sub(1);
                        self.waste_card_indexes.truncate(final_length);
                        let last_waste_card_opt = self.waste_card_indexes.last();
                        if last_waste_card_opt.is_some() {
                            let last_card = *last_waste_card_opt.unwrap();
                            self.waste_area_image_index = Some(last_card);
                            self.cards_moved_from_waste += 1;
                        }
                    }
                    return
                }

                let last_card_index = last_tab_card_index_opt.unwrap();
                let waste_card_index = waste_card_index_opt.unwrap();
                let last_card = &self.cards[last_card_index];
                let waste_card = &self.cards[*waste_card_index];

                // check the value and color
                if last_card.value == waste_card.value + 1 && last_card.color != waste_card.color {
                    self.cards[*waste_card_index].visible = true;
                    self.tab_card_indexes[tab_index].push(*waste_card_index);
                    let final_length = self.waste_card_indexes.len().saturating_sub(1);
                    self.waste_card_indexes.truncate(final_length);
                    let last_waste_card_opt = self.waste_card_indexes.last();
                    if last_waste_card_opt.is_some() {
                        let last_card = *last_waste_card_opt.unwrap();
                        self.waste_area_image_index = Some(last_card);
                    }
                    self.cards_moved_from_waste += 1;
                    return
                } else {
                    return
                }
            }
            
        }

        pub fn move_waste_to_foundation(&mut self, fd_index: usize) {
            let waste_card_index_opt = self.waste_card_indexes.last();
            let waste_card_index = match waste_card_index_opt {
                Some(index) => *index,
                None => return,
            };
            
            // validate move ace to empty fd
            // fd index of 100 indicates empty 
            if self.foundation_cards[fd_index] == 100 && self.cards[waste_card_index].value != 1 {
                return
            }
            
            if self.foundation_cards[fd_index] != 100 {
                let fd_card_index = self.foundation_cards[fd_index];
                // value waste must be 1 more than fd card
                if self.cards[fd_card_index].value != self.cards[waste_card_index].value-1 {
                    return
                }
                // colors must equal
                if self.cards[fd_card_index].color != self.cards[waste_card_index].color {
                    return
                }

                if self.cards[fd_card_index].suite != self.cards[waste_card_index].suite {
                    return
                } 
            }
            
            // move card   
            self.foundation_cards[fd_index] = waste_card_index;
            let final_length = self.waste_card_indexes.len().saturating_sub(1);
            self.waste_card_indexes.truncate(final_length);
            let last_waste_card_opt = self.waste_card_indexes.last();
            if last_waste_card_opt.is_some() {
                self.waste_area_image_index = Some(*last_waste_card_opt.unwrap());
            } else {
                self.waste_area_image_index = None;
            }
            self.cards_moved_from_waste += 1;
            
        }

        fn move_tab_to_tab(&mut self, (tab_index_from, from_index): (usize, usize), 
                                        (tab_index_to, _to_index): (usize, usize)) {
            // if the selected card is last, just move it
            // if the selected card is not last it means we are moving many cards
            // The move to will always be appending to the tab
            let from_len = self.tab_card_indexes[tab_index_from].len();
            let moving_last = if from_len-1 == from_index {
                true
            } else {
                false
            };

            let card_from_index_opt: Option<&usize> = if moving_last {
                self.tab_card_indexes[tab_index_from].last()
            } else {
                Some(&self.tab_card_indexes[tab_index_from][from_index])
            };
            // can unwrap due to the check in the calling method
            let card_from_index = *card_from_index_opt.unwrap();
        
            let card_to_index_opt = self.tab_card_indexes[tab_index_to].last();

            // move if king
            if card_to_index_opt.is_none() && self.cards[card_from_index].value == 13 {
                //continue
            } else {
                let card_to_index = *card_to_index_opt.unwrap();
                if self.cards[card_from_index].color == self.cards[card_to_index].color {
                    return
                } else {
                    if self.cards[card_from_index].value+1 != self.cards[card_to_index].value {
                        return
                    }
                }
            }

            // remove last card from
            if moving_last {
                let final_length = self.tab_card_indexes[tab_index_from].len().saturating_sub(1);
                self.tab_card_indexes[tab_index_from].truncate(final_length);
                self.tab_card_indexes[tab_index_to].push(card_from_index);
                
            } else {
                // move many
                let moving_indexes = self.tab_card_indexes[tab_index_from].split_off(from_index);
                self.tab_card_indexes[tab_index_to].extend(moving_indexes);
            }
            

            // turn over the last card if not empty
            let card_opt = self.tab_card_indexes[tab_index_from].last();
            match card_opt {
                Some(index) => self.cards[*index].visible = true,
                None => (),
            }

        }

        pub fn move_tab_to_foundation(&mut self, tab_index: usize, fd_index: usize) {
            let tab_card_index_opt = self.tab_card_indexes[tab_index].last();
            let tab_card_index = match tab_card_index_opt {
                Some(index) => *index,
                None => return,
            };
 
            // validate move ace to empty fd
            // fd index of 100 indicates empty 
            if fd_index == 100 && self.cards[tab_card_index].value != 1 {
                return
            }
            
            if self.foundation_cards[fd_index] != 100 {
                let fd_card_index = self.foundation_cards[fd_index];
                // value fd must be 1 less than waste card
                if self.cards[fd_card_index].value != self.cards[tab_card_index].value-1 {
                    return
                }
                // colors must equal
                if self.cards[fd_card_index].color != self.cards[tab_card_index].color {
                    return
                }

                if self.cards[fd_card_index].suite != self.cards[tab_card_index].suite {
                    return
                } 
            }
            // move card
            self.foundation_cards[fd_index] = tab_card_index;
            let final_length = self.tab_card_indexes[tab_index].len().saturating_sub(1);
            self.tab_card_indexes[tab_index].truncate(final_length);
            // show last card
            let last_card_index_opt = self.tab_card_indexes[tab_index].last();
            match last_card_index_opt {
                Some(index) => self.cards[*index].visible = true,
                None => ()
            }
            
        }

        pub fn recalc_tab_positions(&mut self) {
            self.tab_positions = vec![vec![]; 7];
            for i in 0..7 {
                if self.tab_card_indexes[i].is_empty() {
                    self.tab_positions[i] = vec![(Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32, 
                                                            y: self.tab_y_offset_pos},
                                                    Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32 + self.card_size_x, 
                                                            y: self.tab_y_offset_pos + self.card_dist_y + self.card_size_y})];
            
                } else {
                 
                    for (j, card_index) in self.tab_card_indexes[i].iter().enumerate() {
                        if self.cards[*card_index].visible {
                            self.tab_positions[i].push((Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32, 
                                                                y: self.tab_y_offset_pos + self.card_dist_y * j as f32},
                                                        Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32 + self.card_size_x, 
                                                                y: self.tab_y_offset_pos + self.card_dist_y + self.card_dist_y * j as f32 + self.card_size_y}));
                        } else {
                            self.tab_positions[i].push((Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32,
                                                                y: self.tab_y_offset_pos + self.card_dist_y * j as f32},
                                                        Point { x: self.tab_x_offset_pos + self.card_dist_x * i as f32 + self.card_size_x, 
                                                                y: self.tab_y_offset_pos + self.card_dist_y + self.card_dist_y * j as f32}));
                        }
                        
                    }
                }
            }  
        }

        fn is_point_in_any_area(&self, point: Point) -> bool {
  
            if point_in_area(point, self.stock_position) {return true}
            if point_in_area(point, self.waste_position) {return true}
            
            for area in self.foundation_positions.iter() {
                if point_in_area(point, *area) {return true}
            }
           
            for area in self.tab_positions.iter() {
                let top_left = area[0].0;
                let mut bottom_right = Point{ x: top_left.x+100.0, y: top_left.y+175.0 };
                if area.len() > 1 {
                    let last = area[area.len()-1];
                    bottom_right = last.1
                }
                
                if point_in_area(point, (top_left, bottom_right)) {return true}
            }

            false
        }
    }

    fn point_in_area(position: Point, area_to: (Point, Point)) -> bool {
        let to = if position.x >= area_to.0.x && position.x <= area_to.1.x {
            if position.y >= area_to.0.y && position.y <= area_to.1.y {
            true
            } else {
                false
            }
        } else {
            false
        };

        to
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Area {
        None,
        Foundation(usize), // foundation index
        Stock,
        Waste,
        Tableau(usize), // tab index
    }

    #[derive(Debug, Clone, Copy)]
    pub struct CardPosition {
        from: Point,
        to: Point,
    }

    #[derive(Debug, Clone, Copy)]
    enum Pending {
        One { from: Point },
        // Two { from: Point, to: Point },
    }

    impl Pending {
        fn draw(
            &self,
            renderer: &Renderer,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
        ) -> Geometry {
            let frame = Frame::new(renderer, bounds.size());
            frame.into_geometry()
        }
    }

    #[derive(Clone, Debug)]
    pub struct Card {
        suite: String,
        color: String,
        value: u32,
        visible: bool,
        image: Handle,
    }


    fn load_cards() -> Vec<Card> {
        let mut cards_ordered: Vec<Card> = Vec::with_capacity(52);

        for suite in ["clubs", "spades", "hearts", "diamonds"] {
            for i in 1..=13 {
                let path = format!("{}/assets/cards/{}/{}.png",
                env!("CARGO_MANIFEST_DIR"), suite, i);
                let color = if suite == "clubs" || suite == "spades" {
                    "black".to_string()
                } else {
                    "red".to_string()
                };
                
                cards_ordered.push(Card { 
                                    suite: suite.to_string(), 
                                    color,  
                                    value: i,
                                    visible: false,
                                    image: Handle::from_path(path),
                });
            }
        }
        // make a random vec
        let mut rand_vec: Vec<u32> = (0..52).collect();
        rand_vec.shuffle(&mut thread_rng());

        let mut cards: Vec<Card> = vec![];
        for n in rand_vec {
            cards.push(cards_ordered[n as usize].clone())
        }

        cards

    }

    struct CardsDraw<'a> {
        state: &'a State,
    }

    impl<'a> canvas::Program<CardPosition> for CardsDraw<'a> {
        type State = Option<Pending>;

        fn update(
            &self,
            state: &mut Self::State,
            event: Event,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> (event::Status, Option<CardPosition>) {
            let Some(cursor_position) = cursor.position_in(bounds) else {
                return (event::Status::Ignored, None);
            };

            match event {
                Event::Mouse(mouse_event) => {
                    let message = match mouse_event {
                        mouse::Event::ButtonPressed(mouse::Button::Left) => {
                            match *state {
                                None => {
                                    if self.state.is_point_in_any_area(cursor_position) {
                                        *state = Some(Pending::One {
                                            from: cursor_position,
                                        });
                                    } else {
                                        *state = None;
                                    }
                                    
                                    None
                                }
                                Some(Pending::One { from }) => {
                                    *state = None;
                                    if self.state.is_point_in_any_area(cursor_position) {
                                        Some(CardPosition {
                                            from,
                                            to: cursor_position,
                                        })
                                    } else {
                                        None
                                    }
                                }
                            }
                        }
                        _ => None,
                    };

                    (event::Status::Captured, message)
                }
                _ => (event::Status::Ignored, None),
            }
        }

        fn draw(
            &self,
            state: &Self::State,
            renderer: &Renderer,
            theme: &Theme,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> Vec<Geometry> {
            if !self.state.start {
                return vec![]
            }
            let mut geometries = vec![];
            
            geometries.push(self.state.table_cache.draw(renderer, bounds.size(), |frame| {
                    frame.fill_rectangle(iced::Point::ORIGIN, frame.size(), Color::BLACK);
                }));

            let size = iced::Size { width: 100.0, height: 150.0 };

            for i in 0..7 {
                geometries.push(self.state.tab_cache[i].draw(renderer, bounds.size(), |frame| {
                    for (j, index) in self.state.tab_card_indexes[i].iter().enumerate() {
                        if self.state.cards[*index].visible {
                            frame.draw_image(
                            Rectangle::new(self.state.tab_positions[i][j].0, size),
                            canvas::Image::new(self.state.cards[*index].image.clone())
                            );
                        } else {
                            frame.draw_image(
                            Rectangle::new(self.state.tab_positions[i][j].0, size),
                            canvas::Image::new(self.state.cover_image.clone())
                            );
                        }
                    }
                }));
                
            }
            
            geometries.push(self.state.stock_cache.draw(renderer, bounds.size(), |frame| {
                if self.state.stock_area_image_index.is_some() {
                    frame.draw_image(
                    Rectangle::new(self.state.stock_position.0, size),
                    canvas::Image::new(self.state.cover_image.clone())
                    );
                } else {
                    
                    let rectangle = Path::rectangle(self.state.stock_position.0, size);

                    let style = Style::Solid(Color::WHITE);

                    let stroke = Stroke{ style, width: 2.0, ..Default::default()};
                    
                    frame.stroke(&rectangle, stroke);
                }
            }));

            geometries.push(self.state.waste_cache.draw(renderer, bounds.size(), |frame| {
                let position = iced::Point { x: 170.0, y: 25.0};
                
                if self.state.waste_area_image_index.is_some() {
                    let index = self.state.waste_area_image_index.unwrap();
                    frame.draw_image(
                        Rectangle::new(position, size),
                        canvas::Image::new(self.state.cards[index].image.clone())
                        );
                } else {
                    let size = iced::Size { width: 100.0, height: 150.0 };
                    let rectangle = Path::rectangle(position, size);

                    let style = Style::Solid(Color::WHITE);

                    let stroke = Stroke{ style, width: 2.0, ..Default::default()};
                    
                    frame.stroke(&rectangle, stroke);
                }   
            }));

            geometries.push(self.state.foundation_cache.draw(renderer, bounds.size(), |frame| {
                for (i, index) in self.state.foundation_cards.iter().enumerate() {
                    if *index < 52 {
                        frame.draw_image(
                        Rectangle::new(self.state.foundation_positions[i].0, size),
                        canvas::Image::new(self.state.cards[self.state.foundation_cards[i]].image.clone())
                        );
                    } else {
                        let size = iced::Size { width: 100.0, height: 150.0 };
                        let rectangle = Path::rectangle(self.state.foundation_positions[i].0, size);

                        let style = Style::Solid(Color::WHITE);

                        let stroke = Stroke{ style, width: 2.0, ..Default::default()};
                        
                        frame.stroke(&rectangle, stroke);
                        
                    }
                }
            }));

            if let Some(pending) = state {
                geometries.push(pending.draw(renderer, theme, bounds, cursor));
                geometries
            } else {
                geometries
            }
        }

        fn mouse_interaction(
            &self,
            state: &Self::State,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> mouse::Interaction {
            if cursor.is_over(bounds) {
                if state.is_some() {
                    mouse::Interaction::Grab
                } else {
                    mouse::Interaction::Pointer
                }
                
            } else {
                mouse::Interaction::default()
            }
        }
    }

    impl Default for State {
        fn default() -> Self {
            Self::new()
        }
    }

    

    #[test]
    fn test_load_cards() {
        let cards = load_cards();

        // search for any duplicates
        for (index, card) in cards.iter().enumerate() {
            for search_index in index+1..cards.len()-1 {
                if cards[search_index].value == card.value && 
                    cards[search_index].suite == card.suite {
                        dbg!(card);
                }
            }
        }
        dbg!("None found");
    }

    #[test]
    fn test_card_position(){
        let position: Point = Point{ x: 150.0, y: 150.0 };
        let area_from: (Point, Point) = (Point{ x: 100.0, y: 100.0 }, Point{ x: 200.0, y: 200.0 });

        let results = point_in_area(position, area_from);

        assert!(results);

        let position: Point = Point{ x: 90.0, y: 90.0 };

        let results = point_in_area(position, area_from);

        assert!(!results);
    }

    #[test]
    fn test_move_cards_stock_to_waste() {
        let mut state = State::new();
        state.start();

        let stock_len = state.stock_card_indexes.len();
        let waste_len = state.waste_card_indexes.len();

        state.move_stock_to_waste();

        let final_stock_len = state.stock_card_indexes.len();
        let final_waste_len = state.waste_card_indexes.len();
        
        assert_eq!(stock_len-3, final_stock_len);
        assert_eq!(waste_len+3, final_waste_len);
    }

    #[test]
    fn test_move_waste_to_stock() {
        let mut state = State::new();
        state.start();

        let stock_len = state.stock_card_indexes.len();
        
        // remove one to have an even number
        state.stock_card_indexes.remove(3);

        for _ in 0..stock_len/3 {
            state.move_stock_to_waste();
        }

        state.move_waste_to_stock();

        let final_stock_len = state.stock_card_indexes.len();
        let final_waste_len = state.waste_card_indexes.len();

        assert_eq!(stock_len-1, final_stock_len);
        assert_eq!(0, final_waste_len);

    }

    #[test]
    fn test_move_waste_to_tab() {
        let mut state = State::new();
        state.start();

        // move 3 cards over to waste first
        state.move_stock_to_waste();

        let top_card_opt = state.waste_card_indexes.last();

        let top_card = *top_card_opt.unwrap();

        let tab_index = 4;
        let before_last_tab_card_opt = state.tab_card_indexes[tab_index].last();

        let before_last_tab_card = *before_last_tab_card_opt.unwrap();

        // setup incoming card for value and color
        let before_card = state.cards[before_last_tab_card].clone();
        let value = before_card.value;
        let color = before_card.color;

        state.cards[top_card].value = value-1;
        if color == "red".to_string() {
            state.cards[top_card].color = "black".to_string();
        } else {
            state.cards[top_card].color = "red".to_string();
        }
        
        state.move_waste_to_tab(tab_index);

        let last_tab_card = state.tab_card_indexes[tab_index].last();
        let card = *last_tab_card.unwrap();
        
        let waste_card = *state.waste_card_indexes.last().unwrap();

        assert_eq!(top_card, card);

        assert_ne!(top_card, waste_card);

    }

    #[test]
    fn test_move_waste_to_foundation() {
        let mut state = State::new();
        state.start();

        // move 3 cards over to waste first
        state.move_stock_to_waste();

        let top_card_opt = state.waste_card_indexes.last();

        let top_card = *top_card_opt.unwrap();

        state.move_waste_to_foundation(0);

        assert_eq!(top_card, state.foundation_cards[0])
    }

    #[test]
    fn test_move_tab_to_tab() {
        let mut state = State::new();
        state.start();

        let tab_index_from = 3;
        let tab_index_to = 4;

        let before_len_from = state.tab_card_indexes[3].len();
        let before_len_to = state.tab_card_indexes[4].len();

        state.move_tab_to_tab((tab_index_from, before_len_from-1), (tab_index_to, before_len_to-1));

        let _after_len_from = state.tab_card_indexes[3].len();
        let after_len_to = state.tab_card_indexes[4].len();

        assert_eq!(before_len_from-1, 3);
        assert_eq!(before_len_to+1, after_len_to)
    }
    
    #[test]
    fn test_tab_positions() {
        let mut state = State::new();
        state.start();

        for (i, tab) in state.tab_positions.iter().enumerate() {
            println!("{}, {:?}", i, tab);
        }

    }

    #[test]
    fn test_point_in_area() {
        let point_1 = Point{ x: 500.0, y: 500.0 };
        let point_2 = Point{ x: 600.0, y: 600.0 };
        let area = (point_1, point_2);
        let point = Point{ x: 510.0, y: 510.0 };

        let found = point_in_area(point, area);
        assert!(found);

        let point = Point{ x: 490.0, y: 490.0 };
        let found = point_in_area(point, area);
        assert!(!found);
    }

    #[test]
    fn test_find_tab_area() {
        let mut state = State::new();
        state.start();

        let mut point = state.tab_positions[5][2].0;

        // make a little offset
        point.x += 10.0;
        point.y += 10.0;
        
        let (tab_index_opt, tab_card_index_opt) = state.find_tab_area(point);
        
        assert_eq!(tab_index_opt, Some(5));
        assert_eq!(tab_card_index_opt, Some(2));
    }

    #[test]
    fn test_find_foundation_area() {
        let mut state = State::new();
        state.start();

        for i in 0..4 {
            let mut point = state.foundation_positions[i].0;
            // make a little offset
            point.x += 10.0;
            point.y += 10.0;

            let results = state.find_foundation_area(point);
  
            assert_eq!(results, Some(i));
        }
    }

    // #[test]
    // fn test_find_card_in_tab() {
    //     let mut state = State::new();
    //     state.start();

    //     let mut tab_point = state.tab_positions[6][3].0;

    //     // make a little offset
    //     tab_point.x += 10.0;
    //     tab_point.y += 10.0;

    //     let card_index = state.find_card_in_tab(6, tab_point).unwrap();

    //     let actual_card_index = state.tab_card_indexes[6][3];

    //     assert_eq!(&state.cards[actual_card_index].value, &state.cards[card_index].value);
    //     assert_eq!(&state.cards[actual_card_index]._suite, &state.cards[card_index]._suite);
    // }

}

