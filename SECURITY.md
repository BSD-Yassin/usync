# Security Policy

## Supported Versions

We actively support the following versions with security updates:

| Version | Supported          |
| ------- | ------------------ |
| Latest stable release | :white_check_mark: |
| Latest nightly release | :white_check_mark: |
| Previous stable release | :white_check_mark: (for 3 months) |
| < Previous stable | :x: |

## Reporting a Vulnerability

If you discover a security vulnerability, please **do not** open a public issue. Instead, please report it privately using one of the following methods:

### Preferred Method: GitHub Security Advisory
1. Go to the [Security tab](https://github.com/BSD-Yassin/usync/security/advisories/new) in this repository
2. Click "Report a vulnerability"
3. Fill out the security advisory form with details about the vulnerability

### Alternative: Email
If you prefer email, you can contact the maintainer directly at: yassin.bousaadi@gmail.com

## What to Include

When reporting a vulnerability, please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)
- Your preferred disclosure timeline

## Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: Within 24-48 hours
  - High: Within 1 week
  - Medium: Within 1 month
  - Low: Next release cycle

## Security Best Practices

When using `usync`:

1. **Verify Checksums**: Always verify SHA256 checksums before using downloaded binaries
2. **Use Official Releases**: Only download from official GitHub releases
3. **Keep Updated**: Regularly update to the latest stable version
4. **Review Permissions**: Be cautious when granting file system permissions
5. **Network Security**: When copying over SSH or S3, ensure connections are properly secured

## Security Features

- **Checksum Verification**: All releases include SHA256 checksums
- **No Automatic Integrity Checks**: `usync` does not automatically verify file integrity after copy operations. Users should verify critical files manually if needed.
- **Secure Protocols**: Supports SSH, HTTPS, and S3 with proper authentication

## Acknowledgments

We appreciate responsible disclosure and will acknowledge security researchers who help improve the security of `usync`.

