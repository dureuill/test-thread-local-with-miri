use std::cell::RefCell;

use thread_local::MostlySend;

mod thread_local;

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::{borrow::BorrowMut, cell::RefCell};

    use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator as _};

    use super::thread_local as my_thread_local;

    #[test]
    fn fully_send() {
        let datastore = my_thread_local::ThreadLocal::new();
        let mut x = Vec::new();
        (0..100u32)
            .into_par_iter()
            .map(|x| {
                let y = datastore.get_or(|| my_thread_local::FullySend(RefCell::new(x)));
                let mut y = y.0.borrow_mut();
                *y += x;
                *y
            })
            .collect_into_vec(&mut x);

        println!("{:?}", x);
    }

    #[test]
    fn fully_send_threadpool() {
        let datastore = my_thread_local::ThreadLocal::new();
        let mut scoped_thread_pool = scoped_threadpool::Pool::new(8);
        scoped_thread_pool.scoped(|s| {
            for i in 0..100 {
                let datastore = &datastore;
                s.execute(move || {
                    let y = datastore.get_or(|| my_thread_local::FullySend(RefCell::new(0)));
                    let mut y = y.0.borrow_mut();
                    let thread_id = format!("{:?}", std::thread::current().id());
                    let thread_id: usize = thread_id["ThreadId(".len()..thread_id.len() - 1]
                        .parse()
                        .unwrap();
                    *y += thread_id + i;
                });
            }
        });

        let res: Vec<_> = datastore.into_iter().map(|f| f.0.into_inner()).collect();

        println!("{res:?}");
    }

    #[test]
    fn mostly_send_threadpool() {
        let bumpstore = my_thread_local::ThreadLocal::new();
        let datastore = my_thread_local::ThreadLocal::new();
        let mut scoped_thread_pool = scoped_threadpool::Pool::new(8);
        {
            let bumpstore = &bumpstore;
            let datastore = &datastore;
            scoped_thread_pool.scoped(|s| {
                for i in 0..100 {
                    s.execute(move || {
                        let bump = bumpstore.get_or(|| bumpalo::Bump::new());
                        let y = datastore
                            .get_or(|| RefCell::new(bumpalo::collections::Vec::new_in(bump)));
                        let mut y = y.borrow_mut();
                        let thread_id = format!("{:?}", std::thread::current().id());
                        let thread_id: usize = thread_id["ThreadId(".len()..thread_id.len() - 1]
                            .parse()
                            .unwrap();
                        y.push(thread_id + i);
                    });
                }
            });
        }

        let res: Vec<_> = datastore.into_iter().map(|f| f.into_inner()).collect();
        println!("{res:?}");
        drop(res);

        for (index, bump) in bumpstore.into_iter().enumerate() {
            println!("bump #{index} allocated {} bytes", bump.allocated_bytes())
        }
    }

    #[test]
    fn fully_send_crate() {
        //let datastore = thread_local::ThreadLocal::new();
        let mut x = Vec::new();
        (0..100u32)
            .into_par_iter()
            .map(|x| {
                /*let y = datastore.get_or(|| RefCell::new(x));
                let mut y = y.borrow_mut();
                *y += x;*/
                let current_thread = rayon::current_thread_index().unwrap();
                x * current_thread as u32
            })
            .collect_into_vec(&mut x);

        println!("{:?}", x);
    }
}

unsafe impl MostlySend for bumpalo::Bump {}
unsafe impl<'a> MostlySend for RefCell<bumpalo::collections::Vec<'a, usize>> {}
