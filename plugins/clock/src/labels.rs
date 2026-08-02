use gtk4::Label;

/// Holds references to the GTK labels used by the clock widget.
///
/// This struct bundles the three labels that make up the clock widget's
/// 4-line layout:
/// - `time_label`: Displays the current time (Line 0, replaces icon line).
/// - `date_label`: Displays the current date in DD.MM.YYYY format (Line 1, main text).
/// - `weekday_label`: Displays the current weekday in German (Line 2, info text).
#[derive(Clone)]
pub(crate) struct ClockLabels {
    /// Label displaying the current time (HH:MM or HH:MM:SS).
    pub(crate) time_label: Label,
    /// Label displaying the current date (DD.MM.YYYY).
    pub(crate) date_label: Label,
    /// Label displaying the current weekday (e.g. "Donnerstag").
    pub(crate) weekday_label: Label,
}
