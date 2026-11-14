mod postgres_support;
mod scoped_support;
use scoped_support::*;

#[test]
fn sqlite_scoped_notes_follow_anchor_pages() {
    let (_temp, _storage, p) = sqlite("notes");
    notes::notes(p);
}
#[test]
fn sqlite_page_appends_are_local() {
    let (_temp, _storage, p) = sqlite("pages");
    pages::pages(p);
}
#[test]
fn sqlite_first_root_is_global_only_when_it_becomes_the_default() {
    let (_temp, _storage, p) = sqlite("roots");
    roots::first_roots(p);
}
#[test]
fn sqlite_first_page_of_a_project_stays_strict() {
    let (_temp, _storage, p) = sqlite("empty");
    roots::empty_project(p);
}
#[test]
#[ignore = "fresh disposable PostgreSQL only"]
fn postgres_scoped_notes_and_pages() {
    let (_, storage) = postgres_support::setup();
    notes::notes(postgres_support::project(&storage, "scoped-notes"));
    pages::pages(postgres_support::project(&storage, "scoped-pages"));
    roots::first_roots(postgres_support::project(&storage, "scoped-roots"));
    roots::empty_project(postgres_support::project(&storage, "scoped-empty"));
}
