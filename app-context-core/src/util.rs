use std::sync::Weak;

pub fn weak_to_ref<'a, T: ?Sized>(weak: &Weak<T>) -> Option<&'a T> {
    if weak.strong_count() == 0 {
        return None;
    }
    let r = unsafe { weak.as_ptr().as_ref() };
    return r;
}
