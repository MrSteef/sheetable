# Sheetable

**Sheetable** is a Rust library that lets you use online spreadsheets as a lightweight database.
The goal is to map typed Rust structs to sheet rows and provide simple CRUD operations without running a traditional DB.

## Why?

- **Zero infra:** persist small/medium datasets without standing up a database.
- **Typed models:** map Rust structs to sheet rows.
- **Portable design:** provider-agnostic core with adapters.

## Use cases

- Quick prototypes and internal tools.
- Small apps or automation where a full DB might not be necessary.
- Data that collaborators can view/edit directly in a spreadsheet.