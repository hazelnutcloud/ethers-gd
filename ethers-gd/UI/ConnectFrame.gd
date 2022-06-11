extends Control

func _on_Button_pressed():
	var accounts = yield($FrameProvider.get_accounts(), "completed")
	print(accounts)
