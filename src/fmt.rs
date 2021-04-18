use crate::opts::Opts;

use chrono::Utc;
use colored::Colorize;
use std::env;
use std::fmt;
use tracing::{field::Field, info_span, trace, Level};
use tracing_core::{Event, Subscriber};
use tracing_subscriber::field::{MakeExt, MakeVisitor, RecordFields, VisitFmt};
use tracing_subscriber::field::{Visit, VisitOutput};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Default)]
struct FmtFilter {
    hide_date: bool,
    hide_fields: bool,
    hide_level: bool,
    hide_spans: bool,
}

impl From<String> for FmtFilter {
    fn from(filter_string: String) -> Self {
        FmtFilter::from(filter_string.as_str())
    }
}

impl<'a> From<&'a str> for FmtFilter {
    fn from(filter_str: &'a str) -> Self {
        let mut filter = Self::default();

        filter_str
            .chars()
            .for_each(|c| match c.to_ascii_lowercase() {
                'd' => filter.hide_date = true,
                'f' => filter.hide_fields = true,
                'l' => filter.hide_level = true,
                's' => filter.hide_spans = true,
                _ => {}
            });

        filter
    }
}

pub fn setup_tracing(opts: &Opts) {
    let span = info_span!("setup-tracing");
    let _enter = span.enter();

    let filter = if let Some(filter) = env::var_os("RUST_LOG") {
        if opts.quiet {
            "".to_string()
        } else {
            filter.to_string_lossy().to_string()
        }
    } else if opts.quiet {
        "pkger=error".to_string()
    } else if opts.debug {
        "pkger=trace".to_string()
    } else {
        "pkger=info".to_string()
    };

    let fmt_filter = if let Some(filter_str) = &opts.hide {
        FmtFilter::from(filter_str.as_str())
    } else {
        FmtFilter::default()
    };

    let fields_fmt = PkgerFieldsFmt::from(&fmt_filter);
    let events_fmt = PkgerEventFmt::from(&fmt_filter);

    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::TRACE)
        .with_env_filter(&filter)
        .fmt_fields(fields_fmt)
        .event_format(events_fmt)
        .init();

    trace!(log_filter = %filter);
    trace!(fmt_filter = ?fmt_filter);
}

/// Fields visitor factory
struct PkgerFields {
    hide_fields: bool,
}

impl<'writer> MakeVisitor<&'writer mut dyn fmt::Write> for PkgerFields {
    type Visitor = PkgerFieldsVisitor<'writer>;

    fn make_visitor(&self, target: &'writer mut dyn fmt::Write) -> Self::Visitor {
        PkgerFieldsVisitor::new(target, self.hide_fields)
    }
}

/// Fields visitor
struct PkgerFieldsVisitor<'writer> {
    writer: &'writer mut dyn fmt::Write,
    err: Option<fmt::Error>,
    hide_fields: bool,
}
impl<'writer> PkgerFieldsVisitor<'writer> {
    pub fn new(writer: &'writer mut dyn fmt::Write, hide_fields: bool) -> Self {
        Self {
            writer,
            err: None,
            hide_fields,
        }
    }
}

impl<'writer> Visit for PkgerFieldsVisitor<'writer> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            if let Err(e) = write!(self.writer, "{:?}", value) {
                self.err = Some(e);
            }
        } else {
            if !self.hide_fields {
                let value = format!("{:#?}", value);
                let field = format!("{}", field);
                if let Err(e) = write!(
                    self.writer,
                    "{}={}",
                    field.truecolor(0xa1, 0xa1, 0xa1),
                    value.truecolor(0x26, 0xbd, 0xb0).italic(),
                ) {
                    self.err = Some(e);
                }
            }
        }
    }
}

impl<'writer> VisitOutput<fmt::Result> for PkgerFieldsVisitor<'writer> {
    fn finish(self) -> fmt::Result {
        if let Some(e) = self.err {
            Err(e)
        } else {
            Ok(())
        }
    }
}

impl<'writer> VisitFmt for PkgerFieldsVisitor<'writer> {
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.writer
    }
}

/// Fields formatter
struct PkgerFieldsFmt {
    delimiter: &'static str,
    hide_fields: bool,
}

impl<'writer> FormatFields<'writer> for PkgerFieldsFmt {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: &'writer mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result {
        let factory = PkgerFields {
            hide_fields: self.hide_fields,
        }
        .delimited(self.delimiter);
        let mut visitor = factory.make_visitor(&mut writer);
        Ok(fields.record(&mut visitor))
    }
}

impl From<&FmtFilter> for PkgerFieldsFmt {
    fn from(filter: &FmtFilter) -> Self {
        let delimiter = if filter.hide_fields { "" } else { ", " };

        PkgerFieldsFmt {
            delimiter,
            hide_fields: filter.hide_fields,
        }
    }
}

struct PkgerEventFmt {
    hide_date: bool,
    hide_level: bool,
    hide_spans: bool,
}

impl From<&FmtFilter> for PkgerEventFmt {
    fn from(filter: &FmtFilter) -> Self {
        Self {
            hide_date: filter.hide_date,
            hide_level: filter.hide_level,
            hide_spans: filter.hide_spans,
        }
    }
}

impl<S, N> FormatEvent<S, N> for PkgerEventFmt
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        if !self.hide_date {
            write!(
                writer,
                "{} ",
                Utc::now().to_rfc3339().bold().truecolor(0x5f, 0x5f, 0x5f)
            )?;
        }
        if !self.hide_level {
            let level = match *event.metadata().level() {
                Level::ERROR => "ERROR".bold().bright_red(),
                Level::WARN => "WARN".bold().bright_yellow(),
                Level::INFO => "INFO".bold().bright_green(),
                Level::DEBUG => "DEBUG".bold().bright_blue(),
                Level::TRACE => "TRACE".bold().bright_magenta(),
            };
            write!(writer, "{} ", level)?;
        }

        ctx.visit_spans::<fmt::Error, _>(|span| {
            if !self.hide_spans {
                write!(writer, "{}", span.name().bold())?;

                let ext = span.extensions();
                let fields = &ext
                    .get::<FormattedFields<N>>()
                    .expect("will never be `None`");

                if !fields.is_empty() {
                    write!(writer, "{}", "{".bold())?;
                    write!(writer, "{}", fields)?;
                    write!(writer, "{}", "}".bold())?;
                }
                write!(writer, "{}", ":".blue())?;
            }

            Ok(())
        })?;

        ctx.field_format().format_fields(writer, event)?;
        writeln!(writer)
    }
}
