use crate::{Context, Game};
use iroh::EndpointId;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use serde::{Deserialize, Serialize};
use crossterm::event::KeyCode;
use rand::seq::SliceRandom;
use std::cmp::Ordering;
use std::collections::BTreeMap;

const INITIAL_CHIPS: u32 = 1000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit { Clubs, Diamonds, Hearts, Spades }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank { 
    Two = 2, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace 
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Card { pub suit: Suit, pub rank: Rank }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PokerPhase { Waiting, PreFlop, Flop, Turn, River, Showdown }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: EndpointId,
    pub chips: u32,
    pub current_bet: u32,
    pub hand: Vec<Card>,
    pub folded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PokerAction { Join, Fold, Call, Raise(u32), StartRound }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokerState {
    pub players: Vec<PlayerInfo>,
    pub deck: Vec<Card>,
    pub community_cards: Vec<Card>,
    pub pot: u32,
    pub current_bet: u32,
    pub turn_idx: usize,
    pub phase: PokerPhase,
    pub log: String,
    pub small_blind: u32,
}

impl Default for PokerState {
    fn default() -> Self {
        Self {
            players: Vec::new(), deck: Vec::new(), community_cards: Vec::new(),
            pot: 0, current_bet: 0, turn_idx: 0,
            phase: PokerPhase::Waiting,
            log: "Wait for both IDs to appear in 'Players', then Host press 'S'".into(),
            small_blind: 10,
        }
    }
}

pub struct PokerGame { 
    is_host: bool, 
    my_id: EndpointId,
    bet_input: String,
}

impl PokerGame {
    pub fn new(is_host: bool, my_id: EndpointId) -> Self { 
        Self { is_host, my_id, bet_input: String::new() } 
    }

    fn create_deck() -> Vec<Card> {
        let mut deck = Vec::new();
        let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
        let ranks = [
            Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven,
            Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace
        ];
        for &s in &suits {
            for &r in &ranks { deck.push(Card { suit: s, rank: r }); }
        }
        deck.shuffle(&mut rand::rng());
        deck
    }
}

impl Game for PokerGame {
    type Action = PokerAction;
    type State = PokerState;

    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Action>, _me: EndpointId) {
        match event.code {
            KeyCode::Char('j') => ctx.send_action(PokerAction::Join),
            KeyCode::Char('s') => ctx.send_action(PokerAction::StartRound),
            KeyCode::Char('f') => ctx.send_action(PokerAction::Fold),
            KeyCode::Char('c') | KeyCode::Char('l') => ctx.send_action(PokerAction::Call),
            // Betting Input
            KeyCode::Char(d) if d.is_digit(10) => {
                if self.bet_input.len() < 5 { self.bet_input.push(d); }
            }
            KeyCode::Backspace => { self.bet_input.pop(); }
            KeyCode::Enter => {
                if let Ok(amt) = self.bet_input.parse::<u32>() {
                    ctx.send_action(PokerAction::Raise(amt));
                }
                self.bet_input.clear();
            }
            _ => {}
        }
    }

    fn handle_action(&self, action: Self::Action, state: &mut Self::State, actor: EndpointId) {
        // --- AUTO-REGISTRATION ---
        if !state.players.iter().any(|p| p.id == actor) {
            state.players.push(PlayerInfo {
                id: actor, chips: INITIAL_CHIPS, current_bet: 0, hand: Vec::new(), folded: false,
            });
            state.log = format!("ID registered: {}...", &actor.to_string()[..8]);
        }

        match action {
            PokerAction::StartRound if self.is_host => {
                if state.players.len() < 2 { return; }
                state.deck = Self::create_deck();
                state.community_cards.clear();
                state.pot = 0;
                state.current_bet = state.small_blind * 2;
                state.phase = PokerPhase::PreFlop;
                state.turn_idx = 0;
                for p in state.players.iter_mut() {
                    p.hand = vec![state.deck.pop().unwrap(), state.deck.pop().unwrap()];
                    p.folded = false;
                    p.current_bet = 0;
                }
                state.log = "Cards Dealt! Blinds Posted.".into();
            }
            _ if state.phase != PokerPhase::Waiting && state.phase != PokerPhase::Showdown => {
                if state.players[state.turn_idx].id != actor { return; }
                match action {
                    PokerAction::Fold => {
                        state.players[state.turn_idx].folded = true;
                        self.advance_turn(state);
                    }
                    PokerAction::Call => {
                        let diff = state.current_bet.saturating_sub(state.players[state.turn_idx].current_bet);
                        let actual = diff.min(state.players[state.turn_idx].chips);
                        state.players[state.turn_idx].chips -= actual;
                        state.players[state.turn_idx].current_bet += actual;
                        state.pot += actual;
                        self.advance_turn(state);
                    }
                    PokerAction::Raise(amt) => {
                        // Standard rule: Raise must be at least double the current bet or +1 big blind
                        if amt > state.current_bet {
                            let diff = amt - state.players[state.turn_idx].current_bet;
                            if state.players[state.turn_idx].chips >= diff {
                                state.players[state.turn_idx].chips -= diff;
                                state.players[state.turn_idx].current_bet = amt;
                                state.current_bet = amt;
                                state.pot += diff;
                                self.advance_turn(state);
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_tick(&self, _dt: u32, state: &mut Self::State) {
        // Ensure Host is always in the player list
        if self.is_host && !state.players.iter().any(|p| p.id == self.my_id) {
            state.players.push(PlayerInfo {
                id: self.my_id, chips: INITIAL_CHIPS, current_bet: 0, hand: Vec::new(), folded: false,
            });
        }
    }

    fn render(&self, frame: &mut ratatui::Frame, state: &Self::State) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(12),   // Table
                Constraint::Length(3), // Hand
                Constraint::Length(3), // Action/Input
                Constraint::Length(3), // Log
            ])
            .split(frame.area());

        let top_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[0]);

        // 1. Table
        let mut table_text = vec![
            Line::from(vec![Span::styled(format!(" POT: {} ", state.pot), Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))]),
            Line::from(format!(" Phase: {:?} | Blinds: {}/{}", state.phase, state.small_blind, state.small_blind * 2)),
            Line::from(" COMMUNITY:"),
        ];
        let community: Vec<Span> = state.community_cards.iter().map(|c| self.format_card(c)).collect();
        table_text.push(Line::from(community));
        frame.render_widget(Paragraph::new(table_text).block(Block::default().borders(Borders::ALL).title("Poker Table")), top_layout[0]);

        // 2. Players
        let mut player_list = Vec::new();
        for (i, p) in state.players.iter().enumerate() {
            let is_me = p.id == self.my_id;
            let mut style = if is_me { Style::default().fg(Color::Green) } else { Style::default() };
            if i == state.turn_idx && state.phase != PokerPhase::Waiting {
                style = style.add_modifier(Modifier::REVERSED).fg(Color::Cyan);
            }
            player_list.push(Line::from(vec![
                Span::styled(format!(" P{}({}): {} ", i+1, &p.id.to_string()[..6], p.chips), style),
                if p.folded { Span::styled(" [Folded] ", Style::default().fg(Color::Red)) } else { Span::raw("") }
            ]));
        }
        frame.render_widget(Paragraph::new(player_list).block(Block::default().borders(Borders::ALL).title("Players")), top_layout[1]);

        // 3. Your Hand
        if let Some(me) = state.players.iter().find(|p| p.id == self.my_id) {
            let hand: Vec<Span> = me.hand.iter().map(|c| self.format_card(c)).collect();
            frame.render_widget(Paragraph::new(Line::from(hand)).block(Block::default().borders(Borders::ALL).title("YOUR HAND")), chunks[1]);
        }

        // 4. Action Box
        let is_my_turn = state.phase != PokerPhase::Waiting && state.phase != PokerPhase::Showdown && state.players.get(state.turn_idx).map(|p| p.id) == Some(self.my_id);
        let border_color = if is_my_turn { Color::Green } else { Color::DarkGray };
        
        let actions = Paragraph::new(format!(" RAISE TO: {}_", self.bet_input))
            .block(Block::default().borders(Borders::ALL).title("Betting Input").border_style(Style::default().fg(border_color)));
        
        let help = Paragraph::new(" [C] Call/Check | [F] Fold | [NumKeys] Type Raise | [Enter] Send | [S] Start (Host Only)")
            .block(Block::default().borders(Borders::ALL).title("Controls"));

        let action_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(chunks[2]);
        
        frame.render_widget(actions, action_layout[0]);
        frame.render_widget(help, action_layout[1]);

        // 5. Log
        frame.render_widget(Paragraph::new(state.log.clone()).block(Block::default().borders(Borders::ALL).title("Status Log")), chunks[3]);
    }
}

impl PokerGame {
    fn advance_turn(&self, state: &mut PokerState) {
        let active = state.players.iter().filter(|p| !p.folded).count();
        if active <= 1 { return self.resolve_showdown(state); }

        state.turn_idx = (state.turn_idx + 1) % state.players.len();
        while state.players[state.turn_idx].folded {
            state.turn_idx = (state.turn_idx + 1) % state.players.len();
        }

        if state.players.iter().filter(|p| !p.folded).all(|p| p.current_bet == state.current_bet) {
            self.next_street(state);
        }
    }

    fn next_street(&self, state: &mut PokerState) {
        match state.phase {
            PokerPhase::PreFlop => { state.phase = PokerPhase::Flop; for _ in 0..3 { state.community_cards.push(state.deck.pop().unwrap()); } }
            PokerPhase::Flop => { state.phase = PokerPhase::Turn; state.community_cards.push(state.deck.pop().unwrap()); }
            PokerPhase::Turn => { state.phase = PokerPhase::River; state.community_cards.push(state.deck.pop().unwrap()); }
            _ => self.resolve_showdown(state),
        }
        for p in state.players.iter_mut() { p.current_bet = 0; }
        state.current_bet = 0;
        state.turn_idx = 0;
    }

    fn resolve_showdown(&self, state: &mut PokerState) {
        state.phase = PokerPhase::Showdown;
        let mut winners = Vec::new();
        let mut best_score = (0, vec![]);

        for (idx, p) in state.players.iter().enumerate() {
            if p.folded { continue; }
            let score = self.evaluate_hand(&p.hand, &state.community_cards);
            match score.0.cmp(&best_score.0) {
                Ordering::Greater => { best_score = score; winners = vec![idx]; }
                Ordering::Equal if score.1 > best_score.1 => { best_score = score; winners = vec![idx]; }
                Ordering::Equal if score.1 == best_score.1 => { winners.push(idx); }
                _ => {}
            }
        }

        let win_amt = state.pot / (winners.len() as u32).max(1);
        for &w in &winners { state.players[w].chips += win_amt; }
        state.pot = 0;
        state.log = format!("Showdown Winner: P{:?}. Hand: {}", winners.iter().map(|i| i+1).collect::<Vec<_>>(), self.score_name(best_score.0));
    }

    fn evaluate_hand(&self, hand: &[Card], community: &[Card]) -> (u32, Vec<u32>) {
        let mut all = [hand, community].concat();
        all.sort_by(|a, b| b.rank.cmp(&a.rank));
        let mut r_counts = BTreeMap::new();
        let mut s_counts = BTreeMap::new();
        for c in &all {
            *r_counts.entry(c.rank).or_insert(0) += 1;
            *s_counts.entry(c.suit).or_insert(0) += 1;
        }
        let is_flush = s_counts.values().any(|&v| v >= 5);
        let mut r_vec: Vec<u32> = r_counts.keys().map(|&r| r as u32).collect();
        r_vec.sort();
        let mut str_hi = 0;
        for w in r_vec.windows(5) { if w[4] == w[0] + 4 { str_hi = w[4]; } }
        let counts: Vec<_> = r_counts.iter().map(|(&r, &c)| (c, r as u32)).collect();
        let q = counts.iter().filter(|(c, _)| *c == 4).map(|(_, r)| *r).next();
        let t = counts.iter().filter(|(c, _)| *c == 3).map(|(_, r)| *r).next();
        let p = counts.iter().filter(|(c, _)| *c == 2).map(|(_, r)| *r).collect::<Vec<_>>();

        if is_flush && str_hi > 0 { (9, vec![str_hi]) }
        else if let Some(r) = q { (8, vec![r]) }
        else if t.is_some() && !p.is_empty() { (7, vec![t.unwrap(), p[0]]) }
        else if is_flush { (6, all.iter().take(5).map(|c| c.rank as u32).collect()) }
        else if str_hi > 0 { (5, vec![str_hi]) }
        else if let Some(r) = t { (4, vec![r]) }
        else if p.len() >= 2 { (3, vec![p[0], p[1]]) }
        else if !p.is_empty() { (2, vec![p[0]]) }
        else { (1, all.iter().take(5).map(|c| c.rank as u32).collect()) }
    }

    fn score_name(&self, s: u32) -> &'static str {
        match s { 9=>"Straight Flush", 8=>"Quads", 7=>"Full House", 6=>"Flush", 5=>"Straight", 4=>"Trips", 3=>"Two Pair", 2=>"Pair", _=>"High Card"}
    }

    fn format_card(&self, card: &Card) -> Span<'_> {
        let (color, suit) = match card.suit {
            Suit::Hearts => (Color::Red, "H"), Suit::Diamonds => (Color::Red, "D"),
            Suit::Clubs => (Color::White, "C"), Suit::Spades => (Color::White, "S"),
        };
        Span::styled(format!(" {:?}{} ", card.rank, suit), Style::default().fg(color).bg(Color::Black))
    }
}