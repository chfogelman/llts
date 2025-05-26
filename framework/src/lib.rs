//use std::ops::Index;
//use std::ops::IndexMut;
//use std::ops::Deref;
//use std::ops::DerefMut;
//
//struct Dag<T>(T);
//
//impl<T> Dag<T> {
//    fn with<M, H: Handler<M>>(self, handler: H) -> (Node<H>, Dag<(H, Self)>) {
//
//        todo!()
//    }
//}
//
//impl<T, U> Deref for Dag<(T, U)> {
//    type Target = U;
//
//    fn deref(&self) -> &Self::Target {
//        &self.0.1
//    }
//
//}
//
////impl<T, U> DerefMut for Dag<(T, U)> {
////    fn deref_mut(&mut self) -> &mut Self::Target {
////        &mut self.0.1
////    }
////}
//
//impl<H, T, U> Index<Node<H>> for Dag<(T, U)>
//{
//    type Output = H;
//
//    fn index(&self, index: Node<H>) -> &Self::Output {
//        index.get(self)
//    }
//}
//
////impl<H, T, U> IndexMut<Node<H>> for Dag<(T, U)> {
////    fn index_mut(&mut self, index: Node<H>) -> &mut Self::Output {
////        todo!()
////    }
////}
//
//#[derive(Clone, Copy)]
//struct Node<H>(std::marker::PhantomData<H>);
//
//impl<H> Node<H> {
//    fn get<U>(self, dag: &Dag<(H, U)>) -> &H {
//        &dag.0.0
//    }
//}
//
//
//trait Handler<Msg> {
//    type Context<U>;
//
//    fn on_message<U>(&mut self, ctx: &mut Self::Context<U>, msg: Msg);
//}
