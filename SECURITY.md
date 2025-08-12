# Security Policy

## ⚠️ Alpha Software Notice

**WFL is currently in alpha stage and should not be used in production environments.** This alpha status means that security features are still being developed and hardened. Use WFL only for development, testing, and educational purposes.

## 🛡️ Supported Versions

We provide security updates for the following versions of WFL:

| Version Pattern | Supported          | Notes |
| --------------- | ------------------ | ----- |
| 25.8.x (Current)| ✅ Yes             | Active development, security fixes prioritized |
| 25.7.x          | ⚠️ Limited         | Critical security issues only |
| 25.6.x and older| ❌ No             | No security updates provided |

**Version Scheme**: WFL uses calendar-based versioning (YY.MM.BUILD). Security patches are released as point releases within the current month.

## 🔒 Reporting Security Vulnerabilities

We take security vulnerabilities seriously and appreciate responsible disclosure from the security community.

### How to Report

**Please DO NOT report security vulnerabilities through public GitHub issues.**

Instead, please use one of these methods:

1. **Preferred**: [GitHub Security Advisories](https://github.com/WebFirstLanguage/wfl/security/advisories/new) (private reporting)
2. **Email**: Send details to `info@logbie.com` with subject line "WFL Security Vulnerability"
3. **Alternative**: Direct message to repository maintainers

### What to Include

Please include the following information in your report:

- **Description**: Clear description of the vulnerability
- **Impact**: Potential security impact and affected components
- **Steps to Reproduce**: Detailed reproduction steps
- **WFL Version**: Specific version where the issue was discovered
- **Environment**: Operating system, Rust version, and relevant configuration
- **Proof of Concept**: Sample WFL code or commands that demonstrate the issue
- **Proposed Solution**: If you have suggestions for fixing the issue

### Response Timeline

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Initial Assessment**: Initial severity assessment within 5 business days
- **Status Updates**: Regular updates on progress toward a fix
- **Resolution**: We aim to provide fixes for critical issues within 30 days
- **Disclosure**: Coordinated public disclosure after fix is available (typically 90 days)

## 🔧 Security Update Process

### Development Process

All security fixes follow our Test-Driven Development (TDD) methodology:

1. **Failing Test Creation**: Security issue reproduction test written first
2. **Implementation**: Minimal fix developed to pass the test
3. **Verification**: All existing tests must continue passing
4. **Review**: Internal security review and testing
5. **Release**: Version bump and coordinated disclosure

### Update Delivery

- **Critical Security Fixes**: Emergency releases outside normal schedule
- **High/Medium Priority**: Included in next scheduled monthly release
- **Low Priority**: May be batched with feature releases

### Notification Channels

- GitHub Security Advisories
- Release notes with security section
- Email notification to registered users (when available)

## 🛠️ WFL-Specific Security Considerations

### Code Execution Safety

WFL interprets and executes user-provided code. Consider these security implications:

**File System Access**:
- WFL programs can read/write files with user permissions
- Use appropriate file system permissions and sandboxing
- Consider running WFL in containerized environments for untrusted code

**Network Operations**:
- WFL supports HTTP requests and database connections
- Validate all network destinations and inputs
- Consider firewall rules and network isolation

**System Integration**:
- WFL can execute system operations through its standard library
- Review WFL programs before execution in sensitive environments
- Monitor resource usage (CPU, memory, network) during execution

### Configuration Security

**Global Configuration** (`/etc/wfl/wfl.cfg` or `C:\wfl\config`):
- Protect global config files with appropriate permissions (readable by WFL users only)
- Regularly review configuration settings
- Use environment variables for sensitive configuration when possible

**Local Configuration** (`.wflcfg` files):
- Keep project-specific configuration in version control
- Avoid storing credentials or sensitive data in configuration files
- Use secure defaults and validate all configuration values

### Development Environment

**Unsafe Code Usage**:
- Limited unsafe Rust code in REPL functionality (`src/repl.rs`)
- Configuration environment manipulation (`src/config.rs`)
- These are audited and necessary for platform functionality

**Dependencies**:
- Regularly update Rust dependencies for security patches
- Monitor dependency security advisories
- Use `cargo audit` for vulnerability scanning

## ⚙️ Security Best Practices for WFL Users

### Running WFL Programs

1. **Source Code Review**: Always review WFL code before execution
2. **Sandboxing**: Consider containerization or virtual machines for untrusted code
3. **Resource Limits**: Monitor CPU, memory, and network usage
4. **File Permissions**: Run with minimal necessary file system permissions
5. **Network Restrictions**: Use firewall rules to limit network access

### WFL Development

1. **Input Validation**: Always validate external inputs in WFL programs
2. **Error Handling**: Use WFL's `try`/`when error` constructs properly
3. **Secure Defaults**: Follow principle of least privilege in your WFL applications
4. **Testing**: Include security testing in your WFL program test suites

### Configuration Management

```ini
# Example secure .wflcfg
timeout_seconds = 30          # Reasonable timeout
logging_enabled = true        # Enable for audit trails
debug_report_enabled = false  # Disable in production-like environments
max_nesting_depth = 5         # Prevent deep recursion attacks
```

## 🔍 Known Security Limitations

As alpha software, WFL has the following known limitations:

1. **Execution Sandboxing**: No built-in sandboxing for untrusted code execution
2. **Resource Limits**: Limited built-in protection against resource exhaustion
3. **Input Sanitization**: Basic input validation - additional sanitization may be needed
4. **Audit Logging**: Security-focused audit logging still in development
5. **Cryptographic Operations**: No built-in cryptographic functions (rely on external tools)

## 📚 Security Resources

### Documentation

- [WFL Architecture](Docs/technical/wfl-architecture-diagram.md) - Understanding system components
- [Error Handling](Docs/language-reference/wfl-errors.md) - Secure error management
- [Async Operations](Docs/language-reference/wfl-async.md) - Network security considerations

### Security Testing

- Use `cargo clippy` for static analysis
- Run `cargo audit` for dependency vulnerability scanning
- Test programs available in `TestPrograms/` directory
- Consider fuzzing WFL programs for robustness testing

### Community Resources

- [GitHub Discussions](https://github.com/WebFirstLanguage/wfl/discussions) - Security questions and best practices
- [Issue Tracker](https://github.com/WebFirstLanguage/wfl/issues) - Non-security bugs and feature requests

## 🤝 Security Acknowledgments

We appreciate the security research community and will acknowledge responsible disclosure contributors:

- Security researchers who report vulnerabilities through proper channels
- Contributors who improve WFL's security posture
- Community members who help identify and document security best practices

## 📞 Contact Information

- **General Security**: info@logbie.com
- **Emergency Security Issues**: Use GitHub Security Advisories for fastest response
- **Project Repository**: https://github.com/WebFirstLanguage/wfl

---

**Last Updated**: August 2025  
**Version**: 25.8.29

© 2025 Logbie LLC. This security policy is subject to updates as WFL evolves from alpha to stable release.