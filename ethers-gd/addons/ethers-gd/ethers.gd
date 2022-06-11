tool
extends EditorPlugin


func _enter_tree():
	add_custom_type("JsonRpcProvider", "Node", preload("res://addons/ethers-gd/classes/json_rpc_provider.gdns"), preload("res://addons/ethers-gd/logos/ethers_logo.png"));
	add_custom_type("AsyncExecuterDriver", "Node", preload("res://addons/ethers-gd/classes/async_executor_driver.gdns"), preload("res://addons/ethers-gd/logos/ethers_logo.png"));


func _exit_tree():
	remove_custom_type("JsonRpcProvider");
	remove_custom_type("AsyncExecuterDriver");
