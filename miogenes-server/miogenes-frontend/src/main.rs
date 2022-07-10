use yew::prelude::*;
use rand::random;

enum Msg {
    Is
}

struct Page {
    view: String,
}

fn fill_randstring(a: &mut String) {
    a.clear();
    for _ in 0..8 {
        a.push(random());
    }
}

impl Component for Page {
    type Message = Msg;

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let mut view = "".to_string();
        fill_randstring(&mut view);
        Page {
            view,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Is => {
                fill_randstring(&mut self.view);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div>
                <button onclick={link.callback(|_| Msg::Is)}>{&self.view}</button>
            </div>
        }
    }

}

fn main() {
    yew::start_app::<Page>();
}
