use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use crate::models::{ArgType, CommandArgs};

#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
    pub suggestion: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n💡 {}", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

pub fn validate_plugin_args(
    provided_args: &HashMap<String, String>,
    command_args: Option<&CommandArgs>,
    plugin_name: &str,
    command_name: &str,
) -> Result<HashMap<String, String>> {
    let Some(args_def) = command_args else {
        // No argument definition means no validation - accept all args (backward compatibility)
        return Ok(provided_args.clone());
    };

    let mut validated_args = HashMap::new();
    let mut errors = Vec::new();

    // Check for required arguments
    for (arg_name, arg_def) in &args_def.required {
        if let Some(value) = provided_args.get(arg_name) {
            match validate_arg_type(value, &arg_def.arg_type) {
                Ok(validated_value) => {
                    validated_args.insert(arg_name.clone(), validated_value);
                }
                Err(e) => {
                    errors.push(format!("Invalid value for required argument '--{}': {}", arg_name, e));
                }
            }
        } else {
            errors.push(format!("Missing required argument '--{}'", arg_name));
        }
    }

    // Check optional arguments and apply defaults
    for (arg_name, arg_def) in &args_def.optional {
        if let Some(value) = provided_args.get(arg_name) {
            match validate_arg_type(value, &arg_def.arg_type) {
                Ok(validated_value) => {
                    validated_args.insert(arg_name.clone(), validated_value);
                }
                Err(e) => {
                    errors.push(format!("Invalid value for optional argument '--{}': {}", arg_name, e));
                }
            }
        } else if let Some(default) = &arg_def.default_value {
            validated_args.insert(arg_name.clone(), default.clone());
        }
    }

    // Check for unknown arguments
    let known_args: HashSet<_> = args_def.required.keys()
        .chain(args_def.optional.keys())
        .collect();
    
    for provided_arg in provided_args.keys() {
        if !known_args.contains(provided_arg) {
            let suggestion = suggest_similar_arg(provided_arg, &known_args);
            let mut error_msg = format!("Unknown argument '--{}' for command '{}:{}'", 
                                      provided_arg, plugin_name, command_name);
            if let Some(suggestion) = suggestion {
                error_msg.push_str(&format!("\n💡 Did you mean '--{}'?", suggestion));
            }
            errors.push(error_msg);
        }
    }

    if !errors.is_empty() {
        let error_msg = format!(
            "❌ Argument validation failed for '{}:{}':\n\n{}",
            plugin_name,
            command_name,
            errors.join("\n")
        );
        
        // Add helpful usage information
        let usage_info = generate_usage_info(args_def, plugin_name, command_name);
        let help_hint = format!("\n💡 For more detailed help, run: mis info {}:{}", plugin_name, command_name);
        return Err(anyhow!("{}\n\n{}{}", error_msg, usage_info, help_hint));
    }

    Ok(validated_args)
}

fn validate_arg_type(value: &str, arg_type: &ArgType) -> Result<String> {
    match arg_type {
        ArgType::String => Ok(value.to_string()),
        ArgType::Boolean => {
            match value.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok("true".to_string()),
                "false" | "0" | "no" | "off" => Ok("false".to_string()),
                _ => Err(anyhow!("expected boolean value (true/false), got '{}'", value)),
            }
        }
        ArgType::Integer => {
            value.parse::<i64>()
                .map(|_| value.to_string())
                .map_err(|_| anyhow!("expected integer value, got '{}'", value))
        }
        ArgType::Float => {
            value.parse::<f64>()
                .map(|_| value.to_string())
                .map_err(|_| anyhow!("expected float value, got '{}'", value))
        }
    }
}

fn suggest_similar_arg(provided: &str, known_args: &HashSet<&String>) -> Option<String> {
    let provided_lower = provided.to_lowercase();
    
    // Look for exact substring matches first
    for &known in known_args {
        let known_lower = known.to_lowercase();
        if known_lower.contains(&provided_lower) || provided_lower.contains(&known_lower) {
            return Some(known.clone());
        }
    }
    
    // Look for args that start with the same letter
    for &known in known_args {
        if known.chars().next() == provided.chars().next() {
            return Some(known.clone());
        }
    }
    
    None
}

fn generate_usage_info(args_def: &CommandArgs, plugin_name: &str, command_name: &str) -> String {
    let mut usage = format!("📖 Usage: mis run {}:{}", plugin_name, command_name);
    
    // Add required args to usage
    for arg_name in args_def.required.keys() {
        usage.push_str(&format!(" --{} <value>", arg_name));
    }
    
    // Add optional args to usage
    for arg_name in args_def.optional.keys() {
        usage.push_str(&format!(" [--{} <value>]", arg_name));
    }
    
    usage.push_str("\n\n📋 Arguments:");
    
    // List required arguments
    if !args_def.required.is_empty() {
        usage.push_str("\n\n  Required:");
        for (name, def) in &args_def.required {
            usage.push_str(&format!("\n    --{:15} {} ({})", 
                                   name, def.description, format_arg_type(&def.arg_type)));
        }
    }
    
    // List optional arguments
    if !args_def.optional.is_empty() {
        usage.push_str("\n\n  Optional:");
        for (name, def) in &args_def.optional {
            let default_info = def.default_value.as_ref()
                .map(|d| format!(" [default: {}]", d))
                .unwrap_or_default();
            usage.push_str(&format!("\n    --{:15} {} ({}){}", 
                                   name, def.description, format_arg_type(&def.arg_type), default_info));
        }
    }
    
    usage
}

fn format_arg_type(arg_type: &ArgType) -> &'static str {
    match arg_type {
        ArgType::String => "string",
        ArgType::Boolean => "boolean",
        ArgType::Integer => "integer",
        ArgType::Float => "float",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ArgDefinition;
    use std::collections::HashMap;

    fn create_test_command_args() -> CommandArgs {
        let mut required = HashMap::new();
        required.insert("name".to_string(), ArgDefinition {
            description: "Name of the item".to_string(),
            arg_type: ArgType::String,
            default_value: None,
        });
        required.insert("count".to_string(), ArgDefinition {
            description: "Number of items".to_string(),
            arg_type: ArgType::Integer,
            default_value: None,
        });

        let mut optional = HashMap::new();
        optional.insert("verbose".to_string(), ArgDefinition {
            description: "Enable verbose output".to_string(),
            arg_type: ArgType::Boolean,
            default_value: Some("false".to_string()),
        });

        CommandArgs { required, optional }
    }

    #[test]
    fn test_validate_plugin_args_success() {
        let mut provided = HashMap::new();
        provided.insert("name".to_string(), "test".to_string());
        provided.insert("count".to_string(), "5".to_string());
        
        let args_def = create_test_command_args();
        let result = validate_plugin_args(&provided, Some(&args_def), "test-plugin", "test-command");
        
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.get("name"), Some(&"test".to_string()));
        assert_eq!(validated.get("count"), Some(&"5".to_string()));
        assert_eq!(validated.get("verbose"), Some(&"false".to_string())); // default applied
    }

    #[test]
    fn test_validate_plugin_args_missing_required() {
        let mut provided = HashMap::new();
        provided.insert("name".to_string(), "test".to_string());
        // Missing 'count' required argument
        
        let args_def = create_test_command_args();
        let result = validate_plugin_args(&provided, Some(&args_def), "test-plugin", "test-command");
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Missing required argument '--count'"));
    }

    #[test]
    fn test_validate_plugin_args_unknown_argument() {
        let mut provided = HashMap::new();
        provided.insert("name".to_string(), "test".to_string());
        provided.insert("count".to_string(), "5".to_string());
        provided.insert("unknown".to_string(), "value".to_string());
        
        let args_def = create_test_command_args();
        let result = validate_plugin_args(&provided, Some(&args_def), "test-plugin", "test-command");
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Unknown argument '--unknown'"));
    }

    #[test]
    fn test_validate_plugin_args_invalid_type() {
        let mut provided = HashMap::new();
        provided.insert("name".to_string(), "test".to_string());
        provided.insert("count".to_string(), "not-a-number".to_string());
        
        let args_def = create_test_command_args();
        let result = validate_plugin_args(&provided, Some(&args_def), "test-plugin", "test-command");
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("expected integer value"));
    }

    #[test]
    fn test_validate_plugin_args_no_definition_backward_compatibility() {
        let mut provided = HashMap::new();
        provided.insert("any-arg".to_string(), "any-value".to_string());
        
        let result = validate_plugin_args(&provided, None, "test-plugin", "test-command");
        
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.get("any-arg"), Some(&"any-value".to_string()));
    }

    #[test]
    fn test_validate_arg_type_boolean() {
        assert_eq!(validate_arg_type("true", &ArgType::Boolean).unwrap(), "true");
        assert_eq!(validate_arg_type("false", &ArgType::Boolean).unwrap(), "false");
        assert_eq!(validate_arg_type("1", &ArgType::Boolean).unwrap(), "true");
        assert_eq!(validate_arg_type("0", &ArgType::Boolean).unwrap(), "false");
        assert_eq!(validate_arg_type("yes", &ArgType::Boolean).unwrap(), "true");
        assert_eq!(validate_arg_type("no", &ArgType::Boolean).unwrap(), "false");
        
        assert!(validate_arg_type("invalid", &ArgType::Boolean).is_err());
    }

    #[test]
    fn test_suggest_similar_arg() {
        let verbose = "verbose".to_string();
        let output = "output".to_string();
        let count = "count".to_string();
        
        let mut known_args = HashSet::new();
        known_args.insert(&verbose);
        known_args.insert(&output);
        known_args.insert(&count);
        
        assert_eq!(suggest_similar_arg("verbos", &known_args), Some("verbose".to_string()));
        assert_eq!(suggest_similar_arg("v", &known_args), Some("verbose".to_string()));
        assert_eq!(suggest_similar_arg("xyz", &known_args), None);
    }
} 