use crate::db::DatabaseManager;
use crate::state::{AppContext, DatabaseSelectPanel, StateTransition, UIState, StateKey};
use crate::state::states::TableListPanel;
use crate::state::states::TableDataViewerState;
use crate::state::states::delete_confirmation_modal::DeleteConfirmationModal;
use crate::state::states::database_name_input_modal::DatabaseNameInputModal;
use crate::state::states::database_save_modal::DatabaseSaveModal;
use crate::state::{ModalKey};
use crossterm::event::KeyEvent;

pub struct StateManager {
    context: AppContext,
    database_select_state: DatabaseSelectPanel,
    table_list_state: TableListPanel,
    table_data_viewer_state: TableDataViewerState,
    // Modal state stack
    modal_stack: Vec<Box<dyn UIState>>,
    // Internal active state tracking
    active_state: StateKey,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            context: AppContext::new(),
            database_select_state: DatabaseSelectPanel::new(),
            table_list_state: TableListPanel::new(),
            table_data_viewer_state: TableDataViewerState::new(),
            modal_stack: Vec::new(),
            active_state: StateKey::DatabaseSelect,
        }
    }
    
    pub fn initialize(&mut self, db_manager: &mut DatabaseManager) {
        // Initialize the starting state
        self.database_select_state.on_enter(&mut self.context, db_manager);
    }

    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        db_manager: &mut DatabaseManager,
    ) -> Option<StateTransition> {
        log::debug!("StateManager handling key: {:?} for state: {:?}", key.code, self.active_state);
        log::debug!("StateManager has {} modals on stack", self.modal_stack.len());
        
        // If there are modals on the stack, handle them first
        if let Some(modal_state) = self.modal_stack.last_mut() {
            let transition = modal_state.handle_event(key, &mut self.context, db_manager);
            log::debug!("Modal got transition: {:?}", transition);
            return self.handle_transition(transition, db_manager);
        }
        
        // Get the transition from the appropriate state
        let transition = match self.active_state {
            StateKey::DatabaseSelect => {
                log::debug!("StateManager: routing to DatabaseSelect state");
                self.database_select_state
                    .handle_event(key, &mut self.context, db_manager)
            }
            StateKey::TableSelect => {
                log::debug!("StateManager: routing to TableSelect state");
                self.table_list_state
                    .handle_event(key, &mut self.context, db_manager)
            }
            StateKey::TableDataViewer => {
                log::debug!("StateManager: routing to TableDataViewer state");
                self.table_data_viewer_state
                    .handle_event(key, &mut self.context, db_manager)
            }
            _ => {
                log::debug!("StateManager: state {:?} not implemented, falling back to legacy", self.active_state);
                return None; // Other states not implemented yet, fall back to legacy handling
            }
        };

        log::debug!("StateManager got transition: {:?}", transition);
        
        // Handle the transition and return StateKey if legacy navigation is needed
        self.handle_transition(transition, db_manager)
    }

    fn handle_transition(&mut self, transition: StateTransition, db_manager: &mut DatabaseManager) -> Option<StateTransition> {
        match transition {
            StateTransition::Exit => Some(StateTransition::Exit),
            StateTransition::To(new_state) => {
                log::debug!("StateManager: handling To({:?}) transition from {:?}", new_state, self.active_state);
                // Handle state transition internally
                if new_state == self.active_state {
                    log::debug!("StateManager: already in target state, returning Stay");
                    return Some(StateTransition::Stay);
                }
                
                // Call on_exit for current state
                match self.active_state {
                    StateKey::DatabaseSelect => {
                        self.database_select_state.on_exit(&mut self.context, db_manager);
                    }
                    StateKey::TableSelect => {
                        self.table_list_state.on_exit(&mut self.context, db_manager);
                    }
                    StateKey::TableDataViewer => {
                        self.table_data_viewer_state.on_exit(&mut self.context, db_manager);
                    }
                    _ => {} // Other states not implemented yet
                }
                
                // Update active state
                let old_state = self.active_state.clone();
                self.active_state = new_state.clone();
                
                // Call on_enter for new state
                match new_state {
                    StateKey::DatabaseSelect => {
                        self.database_select_state.on_enter(&mut self.context, db_manager);
                    }
                    StateKey::TableSelect => {
                        self.table_list_state.on_enter(&mut self.context, db_manager);
                    }
                    StateKey::TableDataViewer => {
                        self.table_data_viewer_state.on_enter(&mut self.context, db_manager);
                    }
                    _ => {
                        // State not implemented in state pattern yet - return for legacy handling
                        return Some(StateTransition::To(new_state));
                    }
                }
                
                log::debug!("StateManager: transitioned from {:?} to {:?}", old_state, new_state);
                
                // State transition handled internally
                Some(StateTransition::Stay)
            }
            StateTransition::Push(modal_key) => {
                // Handle modal push by adding to stack
                let mut modal_state: Box<dyn UIState> = match modal_key {
                    ModalKey::DeleteConfirmation(delete_target) => {
                        Box::new(DeleteConfirmationModal::new(delete_target))
                    }
                    ModalKey::DatabaseNameInput => {
                        Box::new(DatabaseNameInputModal::new())
                    }
                    ModalKey::DatabaseSave => {
                        Box::new(DatabaseSaveModal::new())
                    }
                };

                // Call on_enter for the modal
                modal_state.on_enter(&mut self.context, db_manager);
                self.modal_stack.push(modal_state);

                Some(StateTransition::Stay) // Modal is now active, stay in current state
            }
            StateTransition::PushState(state_key) => {
                // Handle state push (e.g., filter input) by adding to stack
                let mut state: Box<dyn UIState> = match state_key {
                    StateKey::ColumnFilterInput(column_name) => {
                        Box::new(crate::state::states::ColumnFilterInputState::new(column_name))
                    }
                    _ => {
                        log::warn!("StateManager: tried to push unsupported state {:?}", state_key);
                        return Some(StateTransition::Stay);
                    }
                };

                // Call on_enter for the state
                state.on_enter(&mut self.context, db_manager);
                self.modal_stack.push(state);

                Some(StateTransition::Stay) // State is now active on stack
            }
            StateTransition::Stay => {
                // Event was handled internally
                Some(StateTransition::Stay)
            }
            StateTransition::Pop => {
                // Handle modal pop by removing from stack
                let popped_state_name = if let Some(mut modal_state) = self.modal_stack.pop() {
                    let name = modal_state.name().to_string();
                    // Call on_exit for the modal being closed
                    modal_state.on_exit(&mut self.context, db_manager);
                    Some(name)
                } else {
                    None
                };

                // Exact copy of legacy behavior (ui/mod.rs lines 284-288):
                // After filter finalize_search() returns true, refresh data WITHOUT limit
                if popped_state_name.as_deref() == Some("ColumnFilterInputState")
                    && self.active_state == StateKey::TableDataViewer {
                    // Refresh table data without limit (false = no limit, matches legacy line 287)
                    self.table_data_viewer_state.refresh_table_data(&mut self.context, db_manager, false);
                }

                Some(StateTransition::Stay) // Return to underlying state
            }
        }
    }

    pub fn render_database_select(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        db_manager: &DatabaseManager,
    ) {
        self.database_select_state
            .render(frame, area, &self.context, db_manager);
    }

    pub fn render_table_list(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        db_manager: &DatabaseManager,
    ) {
        self.table_list_state
            .render(frame, area, &self.context, db_manager);
    }

    pub fn render_table_data_viewer(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        db_manager: &DatabaseManager,
    ) {
        self.table_data_viewer_state
            .render(frame, area, &self.context, db_manager);
    }

    pub fn is_filter_input_active(&self) -> bool {
        // Check if the top of the modal stack is a ColumnFilterInputState
        if let Some(top_state) = self.modal_stack.last() {
            top_state.name() == "ColumnFilterInputState"
        } else {
            false
        }
    }

    pub fn render_filter_input(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        db_manager: &DatabaseManager,
    ) {
        // Render the filter input if it's on the stack
        if let Some(filter_state) = self.modal_stack.last() {
            filter_state.render(frame, area, &self.context, db_manager);
        }
    }

    pub fn get_context(&self) -> &AppContext {
        &self.context
    }

    pub fn get_context_mut(&mut self) -> &mut AppContext {
        &mut self.context
    }
    
    pub fn get_active_state(&self) -> &StateKey {
        &self.active_state
    }

    fn handle_state_transition(&mut self, transition: StateTransition, _db_manager: &mut DatabaseManager) {
        match transition {
            StateTransition::Stay => {
                // Do nothing, keep current state
            }
            StateTransition::Exit => {
                // This will be handled by the main application
                // For now, we can't directly quit from here
            }
            StateTransition::To(_) => {
                // Panel navigation transitions are handled by the main application
                // This should not be called for To() transitions as they are passed up
            }
            StateTransition::Push(_) => {
                // Modal push transitions are handled by the main application
                // This should not be called for Push() transitions as they are passed up
            }
            StateTransition::PushState(_) => {
                // State push transitions are handled by the main application
                // This should not be called for PushState() transitions as they are passed up
            }
            StateTransition::Pop => {
                todo!("Modal state stack pop handling")
            }
        }
    }


    pub fn sync_notifications(&mut self, notifications: &[crate::app::state::Notification]) {
        self.context.notifications = notifications.to_vec();
    }

    pub fn take_notifications(&mut self) -> Vec<crate::app::state::Notification> {
        std::mem::take(&mut self.context.notifications)
    }

    pub fn sync_all_states(&mut self, db_manager: &DatabaseManager) {
        // Call sync on all existing states
        self.database_select_state.sync(&self.context, db_manager);
        self.table_list_state.sync(&self.context, db_manager);
        self.table_data_viewer_state.sync(&self.context, db_manager);
    }
    
    pub fn enter_state(&mut self, state_key: StateKey, db_manager: &mut DatabaseManager) {
        if state_key == self.active_state {
            return; // Already in this state
        }
        
        self.active_state = state_key.clone();
        
        // Call on_enter for the new state
        match state_key {
            StateKey::DatabaseSelect => {
                self.database_select_state.on_enter(&mut self.context, db_manager);
            }
            StateKey::TableSelect => {
                self.table_list_state.on_enter(&mut self.context, db_manager);
            }
            StateKey::TableDataViewer => {
                self.table_data_viewer_state.on_enter(&mut self.context, db_manager);
            }
            _ => {} // Other states not implemented in state pattern yet
        }
        
        log::debug!("StateManager: entered state {:?}", state_key);
    }
    
    pub fn exit_state(&mut self, state_key: StateKey, db_manager: &mut DatabaseManager) {
        if state_key != self.active_state {
            return; // Not currently in this state
        }
        
        // Call on_exit for the current state
        match state_key {
            StateKey::DatabaseSelect => {
                self.database_select_state.on_exit(&mut self.context, db_manager);
            }
            StateKey::TableSelect => {
                self.table_list_state.on_exit(&mut self.context, db_manager);
            }
            StateKey::TableDataViewer => {
                self.table_data_viewer_state.on_exit(&mut self.context, db_manager);
            }
            _ => {} // Other states not implemented in state pattern yet
        }
        
        log::debug!("StateManager: exited state {:?}", state_key);
    }
    
    pub fn has_active_modal(&self) -> bool {
        !self.modal_stack.is_empty()
    }
    
    pub fn render_top_modal(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        db_manager: &DatabaseManager,
    ) {
        if let Some(modal_state) = self.modal_stack.last() {
            modal_state.render(frame, area, &self.context, db_manager);
        }
    }
}
