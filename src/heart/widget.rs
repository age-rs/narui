/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use crate::heart::*;
use derivative::Derivative;
use lyon::path::Path;
use parking_lot::Mutex;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::replace,
    sync::Arc,
};
use stretch::{geometry::Size, number::Number, style::Style};
use crate::hooks::Listenable;
use std::fmt::Formatter;

/*
The general flow of a frame in narui:
Evaluation -> Layout -> Rendering

1. Evaluation
the output of this Stage is a tree of LayoutObjects

2. Layout
the outputs of this stage are PositionedRenderObjects

3. Rendering
the output of this stage is the visual output :). profit!

 */


pub type Widget = EvalObject;
// The data structure that is input into the Evaluator Pass. When a EvalObject has both
// a layout_object and children, the children are the children of the LayoutObject
#[derive(Clone, Default, Derivative)]
#[derivative(Debug)]
pub struct EvalObject {
    #[derivative(Debug = "ignore")]
    pub children: Vec<(KeyPart, Arc<dyn Fn(Context) -> EvalObject + Send + Sync>)>,
    pub layout_object: Option<LayoutObject>,
}
impl Into<Vec<(KeyPart, Arc<dyn Fn(Context) -> EvalObject + Send + Sync>)>> for EvalObject {
    fn into(self) -> Vec<(KeyPart, Arc<dyn Fn(Context) -> EvalObject + Send + Sync>)> {
        vec![(KeyPart::Nop, Arc::new(move |context| self.clone()))]
    }
}

// A part of the layout tree additionally containing information to render the object
// A LayoutObject is analog to a stretch Node
// but additionally contains a list of RenderObject that can then be passed
// to the render stage.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct LayoutObject {
    pub style: Style,
    #[derivative(Debug = "ignore")]
    pub measure_function: Option<Arc<dyn Fn(Size<Number>) -> Size<f32> + Send + Sync>>,
    pub render_objects: Vec<RenderObject>,
}

pub type PathGenInner = Arc<dyn (Fn(Size<f32>) -> Path) + Send + Sync>;
pub type PathGen = Listenable<PathGenInner>;
// RenderObject is the data structure that really defines _what_ is rendered
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    Path {
        #[derivative(Debug = "ignore")]
        path_gen: PathGen,
        color: Color,
    },
    Text {
        text: String,
        size: f32,
        color: Color,
    },
    Input {
        // this is nothing that gets rendered but instead it gets interpreted by the input handling
        // logic
        #[derivative(Debug = "ignore")]
        on_click: Arc<dyn Fn(Context, bool) + Send + Sync>,
        #[derivative(Debug = "ignore")]
        on_hover: Arc<dyn Fn(Context, bool) + Send + Sync>,
        #[derivative(Debug = "ignore")]
        on_move: Arc<dyn Fn(Context, Vec2) + Send + Sync>,
    },
}
