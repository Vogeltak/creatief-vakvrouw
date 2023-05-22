# Creatief Vakvrouw automated administration tools

Automate the financial administration of Noemi's business.
This includes the following tasks:

- Generate invoices (prefer external sources above manual input)
- Prepare BTW-aangifte
- Keep track of balance sheet
- Keep track of income and costs

## Getting started

1. Check-out source code
2. Dump `clients.json` in the root, e.g.:
```json
  {
    "client-key": {
      "name": "Client A",
      "address": "Main Road 12",
      "zip": "1234AB, Amsterdam"
    }
  }
```
3. Set `LINDA_AUTH` environment variable to a valid session key
4. Run development server with `cargo run -- server`

## Ideas

- Use [WeasyPrint](https://doc.courtbouillon.org/weasyprint/stable/) instead of LaTeX to create PDF invoices
- Use [Typst](https://github.com/typst/typst) typesetting system instead of LaTeX
