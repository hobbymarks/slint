// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use slint::Model;
use slint::VecModel;
use std::cell::RefCell;
use std::rc::Rc;

slint::slint!(export { MainWindow } from "circledraw.slint";);

enum CircleChange {
    Added { row: usize },
    Removed { row: usize, circle: Circle },
    Resized { row: usize, old_d: f32 },
}

struct UndoStack<F> {
    stack: Vec<Option<CircleChange>>,
    // Everything at and after this is a redo action
    redo_offset: usize,
    undo2redo: F,
}

impl<F> UndoStack<F>
where
    F: Fn(CircleChange) -> CircleChange,
{
    fn new(undo2redo: F) -> Self {
        Self { stack: Vec::new(), redo_offset: 0, undo2redo }
    }

    fn push(&mut self, change: CircleChange) {
        self.stack.truncate(self.redo_offset);
        self.stack.push(Some(change));
        self.redo_offset += 1;
    }

    fn undoable(&self) -> bool {
        self.redo_offset > 0
    }

    fn redoable(&self) -> bool {
        self.redo_offset < self.stack.len()
    }

    fn undo(&mut self) {
        self.redo_offset -= 1;

        let undo = self.stack.get_mut(self.redo_offset).unwrap().take().unwrap();
        let redo = (self.undo2redo)(undo);
        self.stack[self.redo_offset] = Some(redo);
    }

    fn redo(&mut self) {
        let redo = self.stack.get_mut(self.redo_offset).unwrap().take().unwrap();
        let undo = (self.undo2redo)(redo);
        self.stack[self.redo_offset] = Some(undo);

        self.redo_offset += 1;
    }
}

pub fn main() {
    let main_window = MainWindow::new().unwrap();

    let model = Rc::new(VecModel::default());
    main_window.set_model(model.clone().into());

    let undo_stack;
    {
        let model = model.clone();
        undo_stack = Rc::new(RefCell::new(UndoStack::new(move |change| match change {
            CircleChange::Added { row } => {
                let circle = model.row_data(row).unwrap();
                model.remove(row);
                CircleChange::Removed { row, circle }
            }
            CircleChange::Removed { row, circle } => {
                model.insert(row, circle);
                CircleChange::Added { row }
            }
            CircleChange::Resized { row, old_d } => {
                let mut circle = model.row_data(row).unwrap();
                let d = circle.d;
                circle.d = old_d;
                model.set_row_data(row, circle);
                CircleChange::Resized { row, old_d: d }
            }
        })));
    }

    {
        let model = model.clone();
        let undo_stack = undo_stack.clone();
        let window_weak = main_window.as_weak();
        main_window.on_background_clicked(move |x, y| {
            let mut undo_stack = undo_stack.borrow_mut();
            let main_window = window_weak.unwrap();

            model.push(Circle { x, y, d: 30.0 });
            undo_stack.push(CircleChange::Added { row: model.row_count() - 1 });

            main_window.set_undoable(undo_stack.undoable());
            main_window.set_redoable(undo_stack.redoable());
        });
    }

    {
        let undo_stack = undo_stack.clone();
        let window_weak = main_window.as_weak();
        main_window.on_undo_clicked(move || {
            let mut undo_stack = undo_stack.borrow_mut();
            let main_window = window_weak.unwrap();
            undo_stack.undo();
            main_window.set_undoable(undo_stack.undoable());
            main_window.set_redoable(undo_stack.redoable());
        });
    }

    {
        let undo_stack = undo_stack.clone();
        let window_weak = main_window.as_weak();
        main_window.on_redo_clicked(move || {
            let mut undo_stack = undo_stack.borrow_mut();
            let main_window = window_weak.unwrap();
            undo_stack.redo();
            main_window.set_undoable(undo_stack.undoable());
            main_window.set_redoable(undo_stack.redoable());
        });
    }

    {
        let model = model.clone();
        let undo_stack = undo_stack.clone();
        let window_weak = main_window.as_weak();
        main_window.on_circle_resized(move |row, diameter| {
            let row = row as usize;
            let mut undo_stack = undo_stack.borrow_mut();
            let main_window = window_weak.unwrap();

            let mut circle = model.row_data(row).unwrap();
            let old_d = circle.d;
            circle.d = diameter;
            model.set_row_data(row, circle);
            undo_stack.push(CircleChange::Resized { row, old_d });

            main_window.set_undoable(undo_stack.undoable());
            main_window.set_redoable(undo_stack.redoable());
        });
    }

    main_window.run().unwrap();
}
