make:
	cargo bundle --target=x86_64-pc-windows-gnu --format=msi 
	cp target/x86_64-pc-windows-gnu/debug/bundle/msi/boredgames.msi ./boredgames.msi