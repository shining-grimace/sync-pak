
- I ought to test handling of connection paths with spaces and special characters
- Backblaze: "region" isn't something clearly visible in the Backblaze UI, so requiring it in the New Provider form needs some explanation; then the custom endpoint seems weird since it appears to include the region within it; also the custom endpoint is buried within advanced settings (should it only be there for AWS S3? Or should region and/or custom endpoint be a property attached to a bucket, hence default bucket of a Provider or a bucket in a Connection? NOTE Backblaze has "custom endpoint" as a property of a bucket!)
- Backblaze not verifying in New Provider form (transport failed before an HTTP response)
- On Android, when an error card is shown at the top (tested on new-provider form), it's partially underneath the status bar, and the close button cannot be tapped
- On Android, add to Settings a one-time purchase of "buy the developer a coffee"
- Update `abiFilters` in `build.gradle.kts` to add x86_64 support needed by ChromeOS
- Android: test that external storage drives work too
- Bump target Android version to API 37
- Set up release config for Android
- Target latest Android
- New setting per connection: approve transfers
- Android connection widget (runs in background if transfers don't use approval)
- Bump Rust and Android dependencies where possible
- Create branding logo
- Review light mode UI theming
- Consider transitions and animations
- Add Google package to AdMob once published on Google Play
- Add app-ads.txt to developer site once requirements are met
- Revise the roadmap's later stages to include prep of marketing material (think what are going to be the most popular workflows, think how they're made easy assuming they are, and how can we market those facts)
- Make sure the roadmap's key points are all implemented
- See where beta versions can be released; Google Play being a likely candidate

In Provider form, advertise the strengths of each provider with a text label underneath the provider selection box, such as:
- Cloudflare: ?
- Backblaze: Secure storage at the lowest cost
- AWS: The company trusted to run most of the world's internet services
