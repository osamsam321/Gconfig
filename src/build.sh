#/bin/bash
if [ `id -u` -ne 0 ]
  then echo Please run this script as root or using sudo!
  exit
fi

main_dir = "/opt/config"
mkdir $main_dir

mkdir $main_dir/backup_config
mkdir $main_dir/storage_config
cp mconfig $main_dir/
cp -r prompt $main_dir/
cp openai_settings.toml $main_dir/
touch $main_dir/config_file_location.json
touch $main_dir/history.txt
ln -s /usr/local/bin/mconfig $main_dir/mconfig
