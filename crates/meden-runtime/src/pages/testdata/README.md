These are disposable test credentials for the isolated DNS/HTTPS fixture.
The private key is intentionally public and has no use outside that fixture.
The leaf certificate covers `*.meden.test` and expires in November 2036.
Only the unit-test client trusts this CA; production uses its normal roots
and must reject this certificate. No certificate verification is disabled.
