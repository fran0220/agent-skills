package service

type Error = ChatError

func AsError(err error) *Error {
	return AsChatError(err)
}
