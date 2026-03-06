use yumeri_animation::animator::Animator;

pub struct EventCtx<'a> {
    pub(crate) animator: &'a mut Animator,
}

impl<'a> EventCtx<'a> {
    pub fn animator(&mut self) -> &mut Animator {
        self.animator
    }
}
