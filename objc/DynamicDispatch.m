#import <Foundation/Foundation.h>
#import <objc/message.h>

id swiftvm_objc_invoke(id target, SEL selector) {
    if (!target || !selector) {
        return nil;
    }
    id (*msgSendTyped)(id, SEL) = (id(*)(id, SEL))objc_msgSend;
    return msgSendTyped(target, selector);
}
