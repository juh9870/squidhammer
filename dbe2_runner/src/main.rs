use std::marker::PhantomData;

trait TestT {
    fn args_count(&self) -> usize;
}

struct FuncNode<Input, Output, F> {
    f: F,
    marker1: PhantomData<fn() -> Input>,
    marker2: PhantomData<fn() -> Output>,
}

trait IntoNode<Input, Output> {
    type Fn: TestT;
    fn into_node(self) -> Self::Fn;
}

impl<O, F: Fn() -> O> IntoNode<(), O> for F {
    type Fn = FuncNode<(), O, F>;

    fn into_node(self) -> Self::Fn {
        FuncNode {
            f: self,
            marker1: Default::default(),
            marker2: Default::default(),
        }
    }
}

impl<I1, O, F: Fn(I1) -> O> IntoNode<(I1,), O> for F {
    type Fn = FuncNode<(I1,), O, F>;

    fn into_node(self) -> Self::Fn {
        FuncNode {
            f: self,
            marker1: Default::default(),
            marker2: Default::default(),
        }
    }
}

impl<I1, I2, O, F: Fn(I1, I2) -> O> IntoNode<(I1, I2), O> for F {
    type Fn = FuncNode<(I1, I2), O, F>;

    fn into_node(self) -> Self::Fn {
        FuncNode {
            f: self,
            marker1: Default::default(),
            marker2: Default::default(),
        }
    }
}

impl<Output, F> TestT for FuncNode<(), Output, F> {
    fn args_count(&self) -> usize {
        1
    }
}
impl<I1, Output, F> TestT for FuncNode<(I1,), Output, F> {
    fn args_count(&self) -> usize {
        1
    }
}
impl<I1, I2, Output, F> TestT for FuncNode<(I1, I2), Output, F> {
    fn args_count(&self) -> usize {
        2
    }
}

pub fn main() {
    args_count(main as fn());
    args_count(main); // the trait bound `fn() {main}: TestT` is not satisfied
    args_count((|x, y| x + y) as fn(usize, usize) -> usize);
    args_count(args_count as fn(fn(usize) -> usize));
}

fn args_count<I, O>(arg: impl IntoNode<I, O>) {
    println!("{}", arg.into_node().args_count());
}
