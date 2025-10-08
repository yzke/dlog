macro_rules! create_function {
    ($func_name:ident) => {
        fn $func_name() {
            // `stringify!` 宏将 `ident` 转换为字符串。
            println!("你调用了 {:?} 函数", stringify!($func_name));
        }
    };
}

// 使用上面的宏创建名为 `foo` 和 `bar` 的函数。
create_function!(foo);
create_function!(bar);

macro_rules! print_result {
    // 这个宏接受一个 `expr` 类型的表达式，
    // 并将其作为字符串打印出来，同时打印其结果。
    // `expr` 指示符用于表达式。
    ($expression:expr) => {
        // `stringify!` 将表达式**原样**转换为字符串。
        println!("{:?} = {:?}",
                 stringify!($expression),
                 $expression);
    };
}

fn main() {
print_result!(1u32 + 1);

    // 记住，代码块也是表达式！
    print_result!({
        let x = 1u32;

        x * x + 2 * x - 1
    });
}