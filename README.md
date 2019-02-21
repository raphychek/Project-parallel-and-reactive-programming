Project template for the "Parallel and reactive programming" course.

This templates include:
 - A fairly extensive API definition in the `src/api` directory.
 - Some common structures and patterns in the `src/common` directory.  These
   should usually be reusable for both sequential and parallel runtimes.  Don't
   hesitate to add your own common helpers there.
 - A sequential runtime implementation in the `src/sequential` directory.  This
   has both a single-use ("dynamic") variant and a reusable ("static") variant.

Some tests, which should also serve as examples, are available in the
`src/lib.rs` file.  You can run them using `cargo test`.
