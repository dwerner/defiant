//! Tests nested packages with `extern_path`.

use defiant::Encode;

include!(concat!(env!("OUT_DIR"), "/extern_paths/packages.rs"));

pub mod widget {
    include!(concat!(env!("OUT_DIR"), "/extern_paths/packages.widget.rs"));
    pub mod factory {
        include!(concat!(
            env!("OUT_DIR"),
            "/extern_paths/packages.widget.factory.rs"
        ));
    }
}

#[test]
fn test() {
    use crate::packages::gizmo;
    use crate::packages::DoIt;

    let arena = defiant::Arena::new();

    let builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    let widget_factory = builder.freeze();
    assert_eq!(0, widget_factory.encoded_len());

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    let widget_factory = builder.freeze();
    assert_eq!(2, widget_factory.encoded_len());

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    builder.set_root(Some(Root {}));
    let widget_factory = builder.freeze();
    assert_eq!(4, widget_factory.encoded_len());

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    builder.set_root(Some(Root {}));
    builder.set_root_inner(Some(root::Inner {}));
    let widget_factory = builder.freeze();
    assert_eq!(6, widget_factory.encoded_len());

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    builder.set_root(Some(Root {}));
    builder.set_root_inner(Some(root::Inner {}));
    builder.set_widget(Some(widget::Widget {}));
    let widget_factory = builder.freeze();
    assert_eq!(8, widget_factory.encoded_len());

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    builder.set_root(Some(Root {}));
    builder.set_root_inner(Some(root::Inner {}));
    builder.set_widget(Some(widget::Widget {}));
    builder.set_widget_inner(Some(widget::widget::Inner {}));
    let widget_factory = builder.freeze();
    assert_eq!(10, widget_factory.encoded_len());

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    builder.set_root(Some(Root {}));
    builder.set_root_inner(Some(root::Inner {}));
    builder.set_widget(Some(widget::Widget {}));
    builder.set_widget_inner(Some(widget::widget::Inner {}));
    builder.set_gizmo(Some(gizmo::Gizmo {}));
    let widget_factory = builder.freeze();
    assert_eq!(12, widget_factory.encoded_len());
    widget_factory.gizmo().map(DoIt::do_it);

    let mut builder = widget::factory::WidgetFactoryBuilder::new_in(&arena);
    builder.set_inner(Some(widget::factory::widget_factory::Inner {}));
    builder.set_root(Some(Root {}));
    builder.set_root_inner(Some(root::Inner {}));
    builder.set_widget(Some(widget::Widget {}));
    builder.set_widget_inner(Some(widget::widget::Inner {}));
    builder.set_gizmo(Some(gizmo::Gizmo {}));
    builder.set_gizmo_inner(Some(gizmo::gizmo::Inner {}));
    let widget_factory = builder.freeze();
    assert_eq!(14, widget_factory.encoded_len());
}
