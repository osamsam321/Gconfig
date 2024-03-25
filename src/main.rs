use std::io::empty;
use std::{fs , };
use std::fs::canonicalize;
use serde_json::{Value, Map, json};
use chrono::Utc;
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
fn main() {
    //parser_json_test();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
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

fn new_config_location_json(file_path:String) -> Result<(), String> {
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
    let mut alias_tmp = file_path.clone();
    if file_path.clone().contains("/"){
        alias_tmp = file_path.clone().split_off(file_path.rfind("/")
            .expect("There was an issue with the slash"));
    }
    //let bkup_loc:String = "backup_config/".to_string().push_str(alias_tmp.as_str());
    // now create a backup of the file location
    let mut bkup_loc:String = "backup_config/".to_string();
    bkup_loc.push_str(alias_tmp.replace("/", "").as_str());

    let new_config_entry = ConfigFile {
        alias: alias_tmp.replace("/", ""),
        iteration: 1,
        realpath: file_path,
        backup_location: bkup_loc,
        ts: chrono::Utc::now().to_string(),
    };

    cfl_content.push(new_config_entry.clone());
    info!("cfl content: {:?}", cfl_content);
    push_file_bkup_dir(new_config_entry.clone());
    //cfl_content.push_str(serde_json::to_string(&new_config_entry_list).expect("unable to push").as_str());
    let json_string: String = serde_json::to_string(&cfl_content).expect("could serialize cfl_content");
    //cfl_content.push(serde_json::json!(new_config_entry).);
    fs::write(cfl, json_string.as_bytes()).expect("could not write to config file histroy");
    info!("writing the follow file metadata to config_file.json {:?}", new_config_entry);
    return Ok(());
}

fn push_file_bkup_dir(config_file_content: ConfigFile){
    let file_content = fs::read_to_string(&config_file_content.realpath).expect("could not create a backup file");
    let mut file_path = "backup_config/".to_string();
    &file_path.push_str(&config_file_content.alias);

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

        let mut final_file_path: String = "".to_string();

        if let Some(file_path) = submatches.get_one::<String>("file_path_to_modify"){
            config_file_content = std::fs::read_to_string(file_path)
                .expect("got string");
            final_file_path = file_path.to_string();
            //new_config_location_file(final_file_path.to_string()).expect("There was an issue with the file");
            new_config_location_json(final_file_path.to_string())
                .expect("There was an issue with the file");
        }
        else if let Some(mut alias) =  submatches.get_one::<String>("alias and nickname"){
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

        let json_req = OpenAiReq{

            model : "gpt-3.5-turbo".to_string(),
            temperature:1.0,
            max_tokens:300,
            top_p:1,
            messages : vec![
                Message{
                    role :"system".to_string(),
                    content: std::fs::read_to_string("prompt_openai.txt").unwrap(),
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
                                                let file_content = fs::read_to_string(&final_file_path).expect("Could not find file path");
                                                info!("old value is {}", value["old"].to_string());

                                                if file_content.contains(&value["old"].as_str().unwrap().to_string())
                                                {
                                                    info!("writing to file");
                                                    fs::write(&final_file_path, file_content.replace(value["old"].as_str().unwrap().to_string().as_str(),
                                                                        value["new"].as_str().unwrap().to_string().as_str()))
                                                                        .expect("there was an issue writing to file");
                                                }

                                            }

                                                let json_s:String = result_json["results"]["change1"]["new"].as_str().unwrap().to_string();
                                                info!("New value: {}", json_s);
                                                let bool = fs::read_to_string(&final_file_path).expect("gone wrong").contains(&json_s.to_string());
                                                info!("\n");
                                                info!("is bool true {}", bool);
                                        } else {
                                             error!("Could not find the 'new' value.");
                                        }


                                        if let Value::Object(map) = &json {
                                                info!("full json {}", json.to_string());
;
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
   println!("All stored files: ");
   println!("");
   for config_content in file_content_json{
       println!("{}" , config_content.alias);
   }
}

async fn parse_cli_arg_matches(matches: ArgMatches){

        match matches.subcommand(){

            Some(("modify_config_file",  modify_file)) => {
               modify_config_file(modify_file.clone()).await;
            }
            Some(("revert_file", revert_file_arg)) => {
                revert_file(revert_file_arg.clone());
            }

            Some(("add_config_file_to_storage", config_location_args)) => {
             let arg_path= Path::new(config_location_args.get_one::<String>("file_path")
                         .map(String::as_str).unwrap()).as_os_str().to_str().unwrap().to_string();
             new_config_location_json(arg_path);
            }

            Some(("list all aliases", alias_list)) => {
                list_aliases(alias_list.clone());
            }

            _ => error!("no arg match"),
        }
}
#[tokio::main]
async fn handle_cli(){
    let matches = Command::new("mconfig")
        .about("adjust your config files using this cli")
        .version("0.0.1")

        .subcommand( Command::new("revert_file") .short_flag('r') .long_flag("revert_file")
                    .about("revert the config file to it's original form")

                    .arg(  Arg::new("file_path_to_revert") .short('f') .long("file_path")
                        .help("use this command to revert the file with the file path")
                        .action(ArgAction::Set) .num_args(1) )

                    .arg(  Arg::new("alias to revert") .short('a') .long("alias to revert")
                        .help("use this command to revert the file using the stored alias/nickname")
                        .action(ArgAction::Set) .num_args(1) )

        )

        .subcommand( Command::new("modify_config_file") .short_flag('m') .long_flag("modify")
                    .about("modify config file")

                    .arg( Arg::new("file_path_to_modify") .short('f') .long("file_path")
                         .action(ArgAction::Set) .num_args(1)
                         .help("Modify the config with the specific file path"))

                    .arg(Arg::new("prompt_change") .short('p').long("prompt")
                            .action(ArgAction::Set) .num_args(1) .required(true)
                            .help("say anything here you wanna change in your specific file"))

                    .arg(Arg::new("alias and nickname") .short('a').long("alias/nickname")
                            .action(ArgAction::Set) .num_args(1)
                            .help("select file from alias/nickname"))


        )

        .subcommand( Command::new("add_config_file_to_storage") .short_flag('a') .long_flag("add")
                    .about("add the config file to storage and create storage")

                    .arg( Arg::new("file_path") .short('f') .long("path")
                         .num_args(1)
                         .help("add the config with the specific file path"))

        )

        .subcommand( Command::new("list all aliases") .short_flag('l') .long_flag("list-alias")
                    .about("list all aliases/nicknames")
        )

        .get_matches();
        parse_cli_arg_matches(matches).await;

}
