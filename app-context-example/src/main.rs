use app_context_derive::AppObj;




#[derive(AppObj)]
#[appobj(export_as="Read,Write")]
pub struct MyStruct{
    #[appobj(autowire="someName")]
    name:String,
    #[appobj(value="hello")]
    msg:String
}



fn main() {
    println!("Hello, world!");
}
