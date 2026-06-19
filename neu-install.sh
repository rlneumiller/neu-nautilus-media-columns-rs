
sudo rm /usr/lib/x86_64-linux-gnu/nautilus/extensions-4/libnautilus4_media_columns_rs.so

sudo cp /home/arrel/gits/neu-nautilus-media-columns-rs/target/release/libnautilus4_media_columns_rs.so /usr/lib/x86_64-linux-gnu/nautilus/extensions-4/
nautilus -q