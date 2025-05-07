# pigweb server

- **[Rocket](https://rocket.rs/) for handling HTTP requests.**
- **[Diesel](https://diesel.rs/) for database, [just use Postgres](https://mccue.dev/pages/8-16-24-just-use-postgres) for the backend.** Postgres supports full text search, so we don't need any additional dependencies for it. See [1](https://admcpr.com/postgres-full-text-search-is-better-than-part1/) [2](https://www.crunchydata.com/blog/postgres-full-text-search-a-search-engine-in-a-database) [3](https://neon.tech/postgresql/postgresql-indexes/postgresql-full-text-search) for implementation guidance.
- **[rocket_oauth2](https://github.com/jebrosen/rocket_oauth2) for SSO.** Used as the base upon which OIDC is implemented.
- **[Figment](https://docs.rs/figment/0.10.19/figment/) for handling configuration.** This is also what Rocket uses, see [here](https://rocket.rs/guide/v0.5/configuration/) for its configuration.
