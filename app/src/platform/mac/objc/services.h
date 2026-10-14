// Our class for handling NSServices messages.
@interface AiTerminalServicesProvider : NSObject
@end

// Functions implemented in Rust.
id ai_terminal_services_provider_custom_url_scheme();
void ai_terminal_app_open_urls(id app, id urls);
