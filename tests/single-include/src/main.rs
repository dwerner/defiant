fn main() {
    let arena = defiant::Arena::new();

    // Use the builder pattern via .builder() on the view type
    let _search_req = {
        let mut builder = single_include::search::SearchRequest::builder(&arena);
        builder.set_query("test");
        builder.set_page_number(1);
        builder.set_result_per_page(10);
        builder.freeze()
    };

    let _out_dir_test = {
        let mut builder = single_include::outdir::OutdirRequest::builder(&arena);
        builder.set_query("test");
        builder.set_page_number(1);
        builder.set_result_per_page(10);
        builder.freeze()
    };
}
