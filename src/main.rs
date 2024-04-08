use std::io::{empty, stdin, self, Write};
use std::{fs , };
use std::fs::canonicalize;
use serde_json::{Value, Map, json};
use chrono::Utc;
use chrono::Local;
use std::collections::HashMap;
use log::{info, trace, warn, error};
use log4rs;
use clap::{Arg, ArgAction, Command, ArgMatches};
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::env;

#[derive(Serialize, Deserialize)]
struct OpenAiReq{
    model: String,
    messages: Vec<Message>,
    temperature:f32,
    max_tokens:u32,
    top_p:u32,

}
#[derive(Serialize, Deserialize, Debug)]
struct ChangeHistory{
    old_change: String,
    new_change: String,
    timestamp: String,
    counter: u32,
}

#[derive(Serialize, Deserialize)]
struct Message{
    role: String,
    content: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ConfigFile{
    alias: String,
    realpath: String,
    iteration: i32,
    backup_location: String,
    ts: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct OpenaiConfig{
    model: Option<String>,
    temperature:Option<f32>,
    max_tokens:Option<u32>,
    top_p:Option<u32>,
}

fn main() {
    //parser_json_test();
    log4rs::init_file("log/log4rs.yaml", Default::default()).unwrap();
    info!("booting up");
    handle_cli();
}

// create the config file that stores all the config file locations ex. "$home/.config/allacritty/allacritty.toml"
fn new_config_location_file(file_path: String) -> std::io::Result<()> {
    let cfl = "config_file_location.txt".to_string();
    let mut file_content = match fs::read_to_string(cfl.clone()) {
        Ok(file) => file,
        Err(error) => match error.kind() {


            std::io::ErrorKind::NotFound => {
                error!("File not found! Attempting to create new file.");
                std::fs::write(cfl, file_path.clone()).expect("Failed trying to create the config file location file");
                return Ok(());
            },
            _ => {
                error!("Issue with the file reading process.");
                return Err(error);
            },
        },
    };
        file_content.push_str("\n");
        file_content.push_str(&file_path.clone());
        std::fs::write(cfl.clone(), &file_content.clone())?;

 Ok(())
}

fn new_config_location_json(file_path: String, mut alias: String) -> Result<(), String> {

    let cfl = "config_file_location.json".to_string();
    let file_content = match fs::read_to_string(&cfl) {
        Ok(file) => file,
        Err(error) => match error.kind(){
            std::io::ErrorKind::NotFound => {
                error!("File not found! Attempting to create new file.");
                std::fs::write(&cfl, "").expect("Failed trying to create the config file location file");
                String::new()
        },
        _ =>{
                error!("Issue with the file reading process. Exiting process");
                return Err(error.to_string());
            },
        },
    };

    let mut cfl_content:Vec<ConfigFile>= match serde_json::from_str(&file_content){
        Ok(content) => content,
        Err(error) => {
            info!("issue with parsing config file {} " , error);
            vec![]
        },
    };

    for content in cfl_content.iter()
    {
        if content.realpath == file_path {
            info!("file content exist. No need to create a entry in the config location file");
            return Ok(());
        }
    }

    info!("file content {} ", file_content);

    let mut bkup_loc:String = "backup_config/".to_string();
    // if no alias is given. The file name will be used as the alias
    if alias.is_empty()
    {
        alias = file_path.clone();
        alias.replace("/", "");
    }

    let mut file_name:String = "".to_string();
    if file_path.clone().contains("/"){
        file_name = file_path.clone().split_off(file_path.clone().rfind("/")
            .expect("There was an issue with the slash"));
    }
    else {
        file_name = file_path.clone();
    }
    //let bkup_loc:String = "backup_config/".to_string().push_str(alias_tmp.as_str());
    // now create a backup of the file location
    bkup_loc.push_str(file_name.replace("/", "").as_str());

    let new_config_entry = ConfigFile {
        alias: alias,
        iteration: 1,
        realpath: file_path.clone(),
        backup_location: bkup_loc,
        ts: chrono::Utc::now().to_string(),
    };

    cfl_content.push(new_config_entry.clone());
    info!("cfl content: {:?}", cfl_content);
    push_file_bkup_dir(new_config_entry.clone());
    //cfl_content.push_str(serde_json::to_string(&new_config_entry_list).expect("unable to push").as_str());
    let json_string: String = serde_json::to_string(&cfl_content).expect("could serialize cfl_content");
    //cfl_content.push(serde_json::json!(new_config_entry).);
    fs::write(cfl, json_string.as_bytes()).expect("could not write to config file history");
    info!("writing the follow file metadata to config_file.json {:?}", new_config_entry);
    return Ok(());
}

fn push_file_bkup_dir(config_file_content: ConfigFile){
    let file_content = fs::read_to_string(&config_file_content.realpath).expect("could not create a backup file");
    let mut file_path = "backup_config/".to_string();
    let file_name = Path::new(&config_file_content.realpath).file_name().and_then(|name| name.to_str()).unwrap_or("");
    &file_path.push_str(file_name);

    match fs::copy(&config_file_content.realpath, file_path.clone()){
        Ok(_) => info!("could not find backup_config directory so a new one was created"),
        Err(_) => {
            fs::create_dir("backup_config").expect("could not create backup config directory");
            fs::copy(config_file_content.realpath.clone(), file_path).expect("retried bkup copy and there was an issue. Exiting!");
        },
    };
}

fn revert_file(submatches:ArgMatches){
     let alias_to_revert = match  submatches.get_one::<String>("alias to revert") {
           Some(alias) => alias,
           None => {
              error!("Error: 'alias was not found in storage. Now Exiting!");
              std::process::exit(1); // exit with an error code
           }
        };
     info!("user request this alias to be reverted {} ", alias_to_revert);
     let config_file_content = fs::read_to_string("config_file_location.json")
        .expect("could not parser config_file_location. Now exiting!");
     let json_content:Vec<ConfigFile> = serde_json::from_str(config_file_content.as_str())
        .expect("could not parser config file to json. Now Exiting");
     info!("checking if alias exist, so it could be reverted");

     for content in json_content.iter() {
         info!("content alias is {} ", content.alias);
        if content.alias == alias_to_revert.clone()  {
            fs::copy(content.backup_location.clone(), content.realpath.clone());
            info!("sucessfully reverted the original file to its backup file");
            break;
        }
     }
}

async fn modify_config_file(submatches:ArgMatches){

         let mut config_file_content = "".to_string();

         let prompt = match  submatches.get_one::<String>("prompt_change") {
           Some(prompt) => prompt,
           None => {
              error!("Error: 'prompt change' not found or not a String.");
              std::process::exit(1); // exit with an error code
           }
        };

         let mut stdin_buf = "".to_string();

         if submatches.get_flag("stdin"){
             for line in io::stdin().lines(){
                 stdin_buf.push_str(&line.unwrap())
             }
         }

        let mut final_file_path: String = "".to_string();

        if let Some(file_path) = submatches.get_one::<String>("file_path_to_modify"){
            config_file_content = std::fs::read_to_string(file_path)
                .expect("got string");
            final_file_path = file_path.to_string();
            //new_config_location_file(final_file_path.to_string()).expect("There was an issue with the file");
            new_config_location_json(final_file_path.to_string(), "".to_string())
                .expect("There was an issue with the file");
        }
        else if let Some(alias) =  submatches.get_one::<String>("alias and nickname"){
            info!("alias was selected");
            let cfl: String= fs::read_to_string("config_file_location.json").
                expect("Could not read from config_file_location.json");
            let cfl_content:Vec<ConfigFile> = serde_json::from_str(cfl.as_str())
                .expect("could serialize the config json file");
            for content in cfl_content.iter(){
               if content.clone().alias == alias.to_string() {
                   final_file_path = content.clone().realpath.trim().to_string();
                    config_file_content = std::fs::read_to_string(final_file_path.clone())
                        .expect("could not read file content. Exiting!");
                   info!("the final_file_path being used is {}", final_file_path);
               }
            }
        }
        else {
            error!("invalid file path");
        }


        let mut content = "Using the config below please ".to_string();
        content.push_str(prompt);
        content.push('\n');
        content.push_str(&config_file_content);
        let openai_config_file_content = fs::read_to_string("openai_settings.toml")
            .expect("could not read or find file");
        let openai_config: OpenaiConfig = toml::from_str(openai_config_file_content.as_str())
            .expect("was unable to parser toml content");

        let json_req = OpenAiReq{

            model : openai_config.model.unwrap() ,
            temperature:openai_config.temperature.unwrap(),
            max_tokens:openai_config.max_tokens.unwrap(),
            top_p:openai_config.top_p.unwrap(),
            messages : vec![
                Message{
                    role :"system".to_string(),
                    content: std::fs::read_to_string("prompt/prompt_openai.txt").
                        expect("could not find prompt file. Exiting!"),
                },
                Message{
                    role: "user".to_string(),
                    content: content.clone(),
                }
            ]
        };

                let openapi_key_env = "OPENAI_API_KEY";
                let openapi_key = match env::var(openapi_key_env){
                    Ok(value) => value,
                    Err(e) => panic!("openapi key is not valid. Exiting Now!"),
                };

                let client = reqwest::Client::new();
                 let response = client.post("https://api.openai.com/v1/chat/completions")
                     .header("Authorization", format!("Bearer {}", openapi_key.to_string()))
                     .json(&json_req)
                     .send()
                     .await;

              match response {
                    Ok(resp) => {
                        match resp.json::<serde_json::Value>().await {
                            Ok(json) => {

                                if let Some(text) = json["choices"][0]["message"]["content"].as_str() {
                                    info!("text full json {}", text);
                                    let result_json: serde_json::Value = serde_json::from_str(json["choices"][0]["message"]["content"].as_str().unwrap()).unwrap();

                                        if let Some(all_results) = result_json["results"].as_object() {

                                            for(key, value) in all_results
                                            {

                                                info!("key {}", key);
                                                info!("value {}", value["old"]);
                                                if submatches.get_flag("stdin"){
                                                    io::stdout().write(stdin_buf.replace(value["old"].as_str().unwrap().to_string().as_str(),
                                                                            value["new"].as_str().unwrap().to_string().as_str()).as_bytes());
                                                }
                                                else {
                                                    let file_content = fs::read_to_string(&final_file_path).expect("Could not find file path");
                                                    info!("old value is {}", value["old"].to_string());

                                                    if file_content.contains(&value["old"].as_str().unwrap().to_string())
                                                    {
                                                        info!("writing to file");
                                                        fs::write(&final_file_path, file_content.replace(value["old"].as_str().unwrap().to_string().as_str(),
                                                                            value["new"].as_str().unwrap().to_string().as_str()))
                                                                            .expect("there was an issue writing to file");
                                                        if submatches.get_flag("show changes"){
                                                            println!("changes: {}", result_json);
                                                        }

                                                    }
                                                }

                                            }

                                                let json_s:String = result_json["results"]["change1"]["new"].as_str().unwrap().to_string();
                                                info!("New value: {}", json_s);
                                        } else {
                                             error!("Could not find the 'new' value.");
                                        }


                                        if let Value::Object(map) = &json {
                                                info!("full json {}", json.to_string());

                                                //let array = json["choices"][0]["message"]["content"][0]["results"].as_str();
                                        }
                                } else {
                                    error!("Something went wrong with the request");
                                    error!("json {}" , json);
                                }
                            },
                            Err(_) => {
                                error!("Error in your json reponse");
                            }
                        }
                    },
                    Err(_) => {
                        error!("There was an error in your json response");

                    }
                }

}

fn list_aliases(submatches:ArgMatches){
   let file_content =  fs::read_to_string("config_file_location.json")
       .expect("Could not read config_file_location.json as text");
   let file_content_json: Vec<ConfigFile> = serde_json::from_str(&file_content.as_str())
       .expect("Could not parse json in config_file_location.json");
   info!("list of all aliases {:?}", file_content_json);
   println!("Your alias and paths: ");
   println!();

   for config_content in file_content_json{
   println!("{0} ->  {1}", config_content.alias, config_content.realpath);
   }
}
// todo! check if the config json and the storage config are both synced
fn check_config_json_synced()
{

}

fn backup_command(submatches: ArgMatches){

 let json_config_content: String = fs::read_to_string("config_file_location.json")
     .expect("unable to open config_file_location.json. Exiting");
 let cf_content: Vec<ConfigFile> = serde_json::from_str(json_config_content.as_str())
     .expect("unable parse the json. Exiting");

 let mut backup_path= "backup_config/".to_string();

    if let Some(file_path_to_revert) = submatches.get_one::<String>("file_path") {
        for cf in cf_content{
            if cf.realpath == file_path_to_revert.to_string(){
                backup_path.push_str(cf.alias.as_str());
                fs::copy(cf.realpath, &backup_path)
                    .expect("unable to copy and paste files. Exiting");
            }
        }
    }
    else if let Some(alias_to_revert) = submatches.get_one::<String>("alias and nickname") {

        for cf in cf_content{
            if cf.alias == alias_to_revert.to_string(){
                backup_path.push_str(cf.alias.as_str());
                fs::copy(cf.realpath, &backup_path)
                    .expect("unable to copy and paste files. Exiting");
            }
        }

    }
    else {
        panic!("Nothing to backup!");
    }
}

fn get_filepath_from_alias(alias_par: String) -> Option<String>{

    let mut file_path = Option::None;
    let cfl_content = fs::read_to_string("config_file_location.json");
    match &cfl_content {
         Ok(file) => file,
         Err(err) => {
            info!("file not found. Attempting to create file ");
            fs::write("config_file_location.json", "")
                .expect("unable to create file. Check permissions, if the file exist, etc");
            &"".to_string()
        },
    };

    let config_content: Vec<ConfigFile> = serde_json::from_str(cfl_content.unwrap().as_str())
        .expect("unable to parser the json here");

    for config_val in config_content {
        if config_val.alias == alias_par{
            if config_val.realpath.trim() != "" {
                file_path = Some(config_val.realpath);
            }
        }
    }

  return Some(file_path)?
}

async fn get_content_to_change_list(args: ArgMatches){
    info!("going to get all possible content");
    let mut config_content = "".to_string();
    if let Some(file_path) = args.get_one::<String>("file_path"){
        config_content = fs::read_to_string(file_path.as_str())
            .expect("There was an issue while trying to read the file");
    }
    else if let Some(alias) = args.get_one::<String>("alias and nickname"){
        config_content = get_filepath_from_alias(alias.to_string()).unwrap();
    }

    let file_content = fs::read_to_string("openai_settings.toml").expect("could not read or find file");
    let openai_config: OpenaiConfig = toml::from_str(file_content.as_str()).expect("was unable to parser toml content");
    let client = reqwest::Client::new();
    let openapi_key = env::var("OPENAI_API_KEY")
        .expect("openapi key not found");
    let json_req = OpenAiReq{

            model : openai_config.model.unwrap(),
            temperature: openai_config.temperature.unwrap(),
            max_tokens:openai_config.max_tokens.unwrap(),
            top_p:openai_config.top_p.unwrap(),
            messages : vec![
                Message{
                    role :"system".to_string(),
                    content: std::fs::read_to_string("prompt/list_content_prompt.txt").
                        expect("could not find prompt file. Exiting!"),
                },
                Message{
                    role: "user".to_string(),
                    content: config_content,
                }
            ]
    };

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openapi_key.to_string()))
        .json(&json_req)
        .send()
        .await;

    match response{

        Ok(response) => {
            println!("Components that could be change: ");
            println!();
            let json_response = response.json::<serde_json::Value>().await.unwrap() ;
            info!("json response {}", json_response);
            if let Some(msg) = json_response["choices"][0]["message"]["content"].as_str(){
                println!("{}", msg);
            }
        },

        Err(err) => println!("There was an error with response"),
    }
}
// ################ utility functions #################

fn get_backup_aliases(){

}

async fn parse_cli_arg_matches(matches: ArgMatches){

        match matches.subcommand(){

            Some(("modify_config_file",  modify_file)) => {
               modify_config_file(modify_file.clone()).await;
            }
            Some(("revert_file", revert_file_arg)) => {
                revert_file(revert_file_arg.clone());
            }

            Some(("add_config_file_to_storage", add_config_file_args)) => {

                let mut alias = "".to_string();
                let file_path = Path::new(add_config_file_args.get_one::<String>("file_path")
                                         .map(String::as_str).unwrap()).as_os_str().to_str().unwrap().to_string();
                if let Some(alias_arg) = add_config_file_args.get_one::<String>("add alias name"){
                        alias = alias_arg.to_string();
                }

                new_config_location_json(file_path, alias);
            }

            Some(("list all aliases", alias_list)) => {
                list_aliases(alias_list.clone());
            }
            Some(("possible config options", possible_configs)) =>{
               get_content_to_change_list(possible_configs.clone()).await;
            }

            Some(("backup", backup_subcommands)) => {
                backup_command(backup_subcommands.clone());
            }
            _ => error!("no arg match"),
        }
}
#[tokio::main]
async fn handle_cli(){
    let matches = Command::new("mconfig")
        .about("adjust your config files using this cli")
        .version("0.0.1")

        .subcommand(Command::new("revert_file") .short_flag('r') .long_flag("revert_file")
                    .about("revert the config file to it's original form")

                    .arg(Arg::new("file_path_to_revert") .short('f') .long("file_path")
                        .help("use this command to revert the file with the file path")
                        .action(ArgAction::Set) .num_args(1) )

                    .arg(Arg::new("alias to revert") .short('a') .long("alias to revert")
                        .help("use this command to revert the file using the stored alias/nickname")
                        .action(ArgAction::Set) .num_args(1) )

        )

        .subcommand(Command::new("modify_config_file") .short_flag('m') .long_flag("modify")
                    .about("modify config file")

                    .arg(Arg::new("file_path_to_modify") .short('f') .long("file_path")
                         .action(ArgAction::Set) .num_args(1)
                         .help("Modify the config with the specific file path"))

                    .arg(Arg::new("prompt_change") .short('p').long("prompt")
                            .action(ArgAction::Set) .num_args(1) .required(true)
                            .help("say anything here you wanna change in your specific file"))

                    .arg(Arg::new("alias and nickname") .short('a').long("alias/nickname")
                            .action(ArgAction::Set) .num_args(1)
                            .help("select file from alias/nickname"))

                    .arg(Arg::new("show changes") .short('d').long("no-output")
                            .action(ArgAction::SetTrue)
                            .help("show changes as output"))

                    .arg(Arg::new("stdin") .short('s').long("stdin")
                            .action(ArgAction::SetTrue)
                            .help("Take stdin as input"))


        )

        .subcommand(Command::new("backup") .short_flag('b') .long_flag("backup")
                    .about("backup file or alias")

                    .arg( Arg::new("file_path") .short('f') .long("file")
                         .num_args(1)
                         .help("add the config with the specific file path"))

                    .arg( Arg::new("alias and nickname") .short('a') .long("alias/nickname")
                         .num_args(1)
                         .help("add the config with the specific file path"))
        )

        .subcommand( Command::new("add_config_file_to_storage") .short_flag('a') .long_flag("add")
                    .about("add the config file to storage and create storage")

                    .arg( Arg::new("file_path") .short('f') .long("path")
                         .num_args(1) .required(true)
                         .help("add the config with the specific file path"))

                    .arg( Arg::new("add alias name") .short('a') .long("alias")
                         .num_args(1)
                         .help("Add and alias name to this config file"))

        )

        .subcommand( Command::new("list all aliases") .short_flag('l') .long_flag("list-alias")
                    .about("list all aliases/nicknames")
        )
        .subcommand( Command::new("possible config options") .short_flag('p') .long_flag("list-possible")
                    .about("list all out all possible configuration changes that could be made")

                    .arg( Arg::new("file_path") .short('f') .long("path")
                         .num_args(1)
                         .help("list possible configurations using the file path"))

                    .arg( Arg::new("alias and nickname") .short('a') .long("alias")
                         .num_args(1)
                         .help("list possible configurations using the alias"))
        )

        .get_matches();
        parse_cli_arg_matches(matches).await;

}
