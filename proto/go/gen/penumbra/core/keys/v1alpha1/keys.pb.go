// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.32.0
// 	protoc        (unknown)
// source: penumbra/core/keys/v1alpha1/keys.proto

package keysv1alpha1

import (
	protoreflect "google.golang.org/protobuf/reflect/protoreflect"
	protoimpl "google.golang.org/protobuf/runtime/protoimpl"
	reflect "reflect"
	sync "sync"
)

const (
	// Verify that this generated code is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(20 - protoimpl.MinVersion)
	// Verify that runtime/protoimpl is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(protoimpl.MaxVersion - 20)
)

// A Penumbra address. An address in Penumbra is a Bech32m-encoded
// string, with the human-readable prefix (HRP) `penumbrav2t`.
type Address struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	// The bytes of the address. Must be represented as a series of
	// `uint8` (i.e. values 0 through 255), with a length of 80 elements.
	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
	// Alternatively, a Bech32m-encoded string representation of the `inner`
	// bytes.
	//
	// NOTE: implementations are not required to support parsing this field.
	// Implementations should prefer to encode the bytes in all messages they
	// produce. Implementations must not accept messages with both `inner` and
	// `alt_bech32m` set.
	AltBech32M string `protobuf:"bytes,2,opt,name=alt_bech32m,json=altBech32m,proto3" json:"alt_bech32m,omitempty"`
}

func (x *Address) Reset() {
	*x = Address{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *Address) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*Address) ProtoMessage() {}

func (x *Address) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use Address.ProtoReflect.Descriptor instead.
func (*Address) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{0}
}

func (x *Address) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

func (x *Address) GetAltBech32M() string {
	if x != nil {
		return x.AltBech32M
	}
	return ""
}

type AddressView struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	// Types that are assignable to AddressView:
	//
	//	*AddressView_Decoded_
	//	*AddressView_Opaque_
	AddressView isAddressView_AddressView `protobuf_oneof:"address_view"`
}

func (x *AddressView) Reset() {
	*x = AddressView{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *AddressView) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*AddressView) ProtoMessage() {}

func (x *AddressView) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use AddressView.ProtoReflect.Descriptor instead.
func (*AddressView) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{1}
}

func (m *AddressView) GetAddressView() isAddressView_AddressView {
	if m != nil {
		return m.AddressView
	}
	return nil
}

func (x *AddressView) GetDecoded() *AddressView_Decoded {
	if x, ok := x.GetAddressView().(*AddressView_Decoded_); ok {
		return x.Decoded
	}
	return nil
}

func (x *AddressView) GetOpaque() *AddressView_Opaque {
	if x, ok := x.GetAddressView().(*AddressView_Opaque_); ok {
		return x.Opaque
	}
	return nil
}

type isAddressView_AddressView interface {
	isAddressView_AddressView()
}

type AddressView_Decoded_ struct {
	Decoded *AddressView_Decoded `protobuf:"bytes,1,opt,name=decoded,proto3,oneof"`
}

type AddressView_Opaque_ struct {
	Opaque *AddressView_Opaque `protobuf:"bytes,2,opt,name=opaque,proto3,oneof"`
}

func (*AddressView_Decoded_) isAddressView_AddressView() {}

func (*AddressView_Opaque_) isAddressView_AddressView() {}

type PayloadKey struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
}

func (x *PayloadKey) Reset() {
	*x = PayloadKey{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[2]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *PayloadKey) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*PayloadKey) ProtoMessage() {}

func (x *PayloadKey) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[2]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use PayloadKey.ProtoReflect.Descriptor instead.
func (*PayloadKey) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{2}
}

func (x *PayloadKey) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

type SpendKey struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
}

func (x *SpendKey) Reset() {
	*x = SpendKey{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[3]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *SpendKey) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*SpendKey) ProtoMessage() {}

func (x *SpendKey) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[3]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use SpendKey.ProtoReflect.Descriptor instead.
func (*SpendKey) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{3}
}

func (x *SpendKey) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

type FullViewingKey struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
}

func (x *FullViewingKey) Reset() {
	*x = FullViewingKey{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[4]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *FullViewingKey) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*FullViewingKey) ProtoMessage() {}

func (x *FullViewingKey) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[4]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use FullViewingKey.ProtoReflect.Descriptor instead.
func (*FullViewingKey) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{4}
}

func (x *FullViewingKey) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

type WalletId struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
}

func (x *WalletId) Reset() {
	*x = WalletId{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[5]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *WalletId) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*WalletId) ProtoMessage() {}

func (x *WalletId) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[5]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use WalletId.ProtoReflect.Descriptor instead.
func (*WalletId) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{5}
}

func (x *WalletId) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

type Diversifier struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
}

func (x *Diversifier) Reset() {
	*x = Diversifier{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[6]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *Diversifier) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*Diversifier) ProtoMessage() {}

func (x *Diversifier) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[6]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use Diversifier.ProtoReflect.Descriptor instead.
func (*Diversifier) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{6}
}

func (x *Diversifier) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

type AddressIndex struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Account    uint32 `protobuf:"varint,2,opt,name=account,proto3" json:"account,omitempty"`
	Randomizer []byte `protobuf:"bytes,3,opt,name=randomizer,proto3" json:"randomizer,omitempty"`
}

func (x *AddressIndex) Reset() {
	*x = AddressIndex{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[7]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *AddressIndex) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*AddressIndex) ProtoMessage() {}

func (x *AddressIndex) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[7]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use AddressIndex.ProtoReflect.Descriptor instead.
func (*AddressIndex) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{7}
}

func (x *AddressIndex) GetAccount() uint32 {
	if x != nil {
		return x.Account
	}
	return 0
}

func (x *AddressIndex) GetRandomizer() []byte {
	if x != nil {
		return x.Randomizer
	}
	return nil
}

// A validator's identity key (decaf377-rdsa spendauth verification key).
type IdentityKey struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Ik []byte `protobuf:"bytes,1,opt,name=ik,proto3" json:"ik,omitempty"`
}

func (x *IdentityKey) Reset() {
	*x = IdentityKey{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[8]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *IdentityKey) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*IdentityKey) ProtoMessage() {}

func (x *IdentityKey) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[8]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use IdentityKey.ProtoReflect.Descriptor instead.
func (*IdentityKey) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{8}
}

func (x *IdentityKey) GetIk() []byte {
	if x != nil {
		return x.Ik
	}
	return nil
}

// A validator's governance key (decaf377-rdsa spendauth verification key).
type GovernanceKey struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Gk []byte `protobuf:"bytes,1,opt,name=gk,proto3" json:"gk,omitempty"`
}

func (x *GovernanceKey) Reset() {
	*x = GovernanceKey{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[9]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *GovernanceKey) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*GovernanceKey) ProtoMessage() {}

func (x *GovernanceKey) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[9]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use GovernanceKey.ProtoReflect.Descriptor instead.
func (*GovernanceKey) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{9}
}

func (x *GovernanceKey) GetGk() []byte {
	if x != nil {
		return x.Gk
	}
	return nil
}

type ConsensusKey struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Inner []byte `protobuf:"bytes,1,opt,name=inner,proto3" json:"inner,omitempty"`
}

func (x *ConsensusKey) Reset() {
	*x = ConsensusKey{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[10]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ConsensusKey) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ConsensusKey) ProtoMessage() {}

func (x *ConsensusKey) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[10]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ConsensusKey.ProtoReflect.Descriptor instead.
func (*ConsensusKey) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{10}
}

func (x *ConsensusKey) GetInner() []byte {
	if x != nil {
		return x.Inner
	}
	return nil
}

// A decoded address, with information about the address index and wallet ID visible.
type AddressView_Decoded struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Address  *Address      `protobuf:"bytes,1,opt,name=address,proto3" json:"address,omitempty"`
	Index    *AddressIndex `protobuf:"bytes,2,opt,name=index,proto3" json:"index,omitempty"`
	WalletId *WalletId     `protobuf:"bytes,3,opt,name=wallet_id,json=walletId,proto3" json:"wallet_id,omitempty"`
}

func (x *AddressView_Decoded) Reset() {
	*x = AddressView_Decoded{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[11]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *AddressView_Decoded) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*AddressView_Decoded) ProtoMessage() {}

func (x *AddressView_Decoded) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[11]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use AddressView_Decoded.ProtoReflect.Descriptor instead.
func (*AddressView_Decoded) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{1, 0}
}

func (x *AddressView_Decoded) GetAddress() *Address {
	if x != nil {
		return x.Address
	}
	return nil
}

func (x *AddressView_Decoded) GetIndex() *AddressIndex {
	if x != nil {
		return x.Index
	}
	return nil
}

func (x *AddressView_Decoded) GetWalletId() *WalletId {
	if x != nil {
		return x.WalletId
	}
	return nil
}

// An opaque address, with no information about the address index or wallet ID visible.
type AddressView_Opaque struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Address *Address `protobuf:"bytes,1,opt,name=address,proto3" json:"address,omitempty"`
}

func (x *AddressView_Opaque) Reset() {
	*x = AddressView_Opaque{}
	if protoimpl.UnsafeEnabled {
		mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[12]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *AddressView_Opaque) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*AddressView_Opaque) ProtoMessage() {}

func (x *AddressView_Opaque) ProtoReflect() protoreflect.Message {
	mi := &file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[12]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use AddressView_Opaque.ProtoReflect.Descriptor instead.
func (*AddressView_Opaque) Descriptor() ([]byte, []int) {
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP(), []int{1, 1}
}

func (x *AddressView_Opaque) GetAddress() *Address {
	if x != nil {
		return x.Address
	}
	return nil
}

var File_penumbra_core_keys_v1alpha1_keys_proto protoreflect.FileDescriptor

var file_penumbra_core_keys_v1alpha1_keys_proto_rawDesc = []byte{
	0x0a, 0x26, 0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2f, 0x63, 0x6f, 0x72, 0x65, 0x2f,
	0x6b, 0x65, 0x79, 0x73, 0x2f, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x2f, 0x6b, 0x65,
	0x79, 0x73, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x1b, 0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62,
	0x72, 0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65, 0x79, 0x73, 0x2e, 0x76, 0x31, 0x61,
	0x6c, 0x70, 0x68, 0x61, 0x31, 0x22, 0x40, 0x0a, 0x07, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73,
	0x12, 0x14, 0x0a, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52,
	0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x12, 0x1f, 0x0a, 0x0b, 0x61, 0x6c, 0x74, 0x5f, 0x62, 0x65,
	0x63, 0x68, 0x33, 0x32, 0x6d, 0x18, 0x02, 0x20, 0x01, 0x28, 0x09, 0x52, 0x0a, 0x61, 0x6c, 0x74,
	0x42, 0x65, 0x63, 0x68, 0x33, 0x32, 0x6d, 0x22, 0xd1, 0x03, 0x0a, 0x0b, 0x41, 0x64, 0x64, 0x72,
	0x65, 0x73, 0x73, 0x56, 0x69, 0x65, 0x77, 0x12, 0x4c, 0x0a, 0x07, 0x64, 0x65, 0x63, 0x6f, 0x64,
	0x65, 0x64, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x30, 0x2e, 0x70, 0x65, 0x6e, 0x75, 0x6d,
	0x62, 0x72, 0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65, 0x79, 0x73, 0x2e, 0x76, 0x31,
	0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x2e, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x56, 0x69,
	0x65, 0x77, 0x2e, 0x44, 0x65, 0x63, 0x6f, 0x64, 0x65, 0x64, 0x48, 0x00, 0x52, 0x07, 0x64, 0x65,
	0x63, 0x6f, 0x64, 0x65, 0x64, 0x12, 0x49, 0x0a, 0x06, 0x6f, 0x70, 0x61, 0x71, 0x75, 0x65, 0x18,
	0x02, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x2f, 0x2e, 0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61,
	0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65, 0x79, 0x73, 0x2e, 0x76, 0x31, 0x61, 0x6c, 0x70,
	0x68, 0x61, 0x31, 0x2e, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x56, 0x69, 0x65, 0x77, 0x2e,
	0x4f, 0x70, 0x61, 0x71, 0x75, 0x65, 0x48, 0x00, 0x52, 0x06, 0x6f, 0x70, 0x61, 0x71, 0x75, 0x65,
	0x1a, 0xce, 0x01, 0x0a, 0x07, 0x44, 0x65, 0x63, 0x6f, 0x64, 0x65, 0x64, 0x12, 0x3e, 0x0a, 0x07,
	0x61, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x24, 0x2e,
	0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65,
	0x79, 0x73, 0x2e, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x2e, 0x41, 0x64, 0x64, 0x72,
	0x65, 0x73, 0x73, 0x52, 0x07, 0x61, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x12, 0x3f, 0x0a, 0x05,
	0x69, 0x6e, 0x64, 0x65, 0x78, 0x18, 0x02, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x29, 0x2e, 0x70, 0x65,
	0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65, 0x79, 0x73,
	0x2e, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x2e, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73,
	0x73, 0x49, 0x6e, 0x64, 0x65, 0x78, 0x52, 0x05, 0x69, 0x6e, 0x64, 0x65, 0x78, 0x12, 0x42, 0x0a,
	0x09, 0x77, 0x61, 0x6c, 0x6c, 0x65, 0x74, 0x5f, 0x69, 0x64, 0x18, 0x03, 0x20, 0x01, 0x28, 0x0b,
	0x32, 0x25, 0x2e, 0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65,
	0x2e, 0x6b, 0x65, 0x79, 0x73, 0x2e, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x2e, 0x57,
	0x61, 0x6c, 0x6c, 0x65, 0x74, 0x49, 0x64, 0x52, 0x08, 0x77, 0x61, 0x6c, 0x6c, 0x65, 0x74, 0x49,
	0x64, 0x1a, 0x48, 0x0a, 0x06, 0x4f, 0x70, 0x61, 0x71, 0x75, 0x65, 0x12, 0x3e, 0x0a, 0x07, 0x61,
	0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x24, 0x2e, 0x70,
	0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65, 0x79,
	0x73, 0x2e, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x2e, 0x41, 0x64, 0x64, 0x72, 0x65,
	0x73, 0x73, 0x52, 0x07, 0x61, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x42, 0x0e, 0x0a, 0x0c, 0x61,
	0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x5f, 0x76, 0x69, 0x65, 0x77, 0x22, 0x22, 0x0a, 0x0a, 0x50,
	0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64, 0x4b, 0x65, 0x79, 0x12, 0x14, 0x0a, 0x05, 0x69, 0x6e, 0x6e,
	0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x22,
	0x20, 0x0a, 0x08, 0x53, 0x70, 0x65, 0x6e, 0x64, 0x4b, 0x65, 0x79, 0x12, 0x14, 0x0a, 0x05, 0x69,
	0x6e, 0x6e, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x05, 0x69, 0x6e, 0x6e, 0x65,
	0x72, 0x22, 0x26, 0x0a, 0x0e, 0x46, 0x75, 0x6c, 0x6c, 0x56, 0x69, 0x65, 0x77, 0x69, 0x6e, 0x67,
	0x4b, 0x65, 0x79, 0x12, 0x14, 0x0a, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01,
	0x28, 0x0c, 0x52, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x22, 0x20, 0x0a, 0x08, 0x57, 0x61, 0x6c,
	0x6c, 0x65, 0x74, 0x49, 0x64, 0x12, 0x14, 0x0a, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x18, 0x01,
	0x20, 0x01, 0x28, 0x0c, 0x52, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x22, 0x23, 0x0a, 0x0b, 0x44,
	0x69, 0x76, 0x65, 0x72, 0x73, 0x69, 0x66, 0x69, 0x65, 0x72, 0x12, 0x14, 0x0a, 0x05, 0x69, 0x6e,
	0x6e, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72,
	0x22, 0x48, 0x0a, 0x0c, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x49, 0x6e, 0x64, 0x65, 0x78,
	0x12, 0x18, 0x0a, 0x07, 0x61, 0x63, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x18, 0x02, 0x20, 0x01, 0x28,
	0x0d, 0x52, 0x07, 0x61, 0x63, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x12, 0x1e, 0x0a, 0x0a, 0x72, 0x61,
	0x6e, 0x64, 0x6f, 0x6d, 0x69, 0x7a, 0x65, 0x72, 0x18, 0x03, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x0a,
	0x72, 0x61, 0x6e, 0x64, 0x6f, 0x6d, 0x69, 0x7a, 0x65, 0x72, 0x22, 0x1d, 0x0a, 0x0b, 0x49, 0x64,
	0x65, 0x6e, 0x74, 0x69, 0x74, 0x79, 0x4b, 0x65, 0x79, 0x12, 0x0e, 0x0a, 0x02, 0x69, 0x6b, 0x18,
	0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x02, 0x69, 0x6b, 0x22, 0x1f, 0x0a, 0x0d, 0x47, 0x6f, 0x76,
	0x65, 0x72, 0x6e, 0x61, 0x6e, 0x63, 0x65, 0x4b, 0x65, 0x79, 0x12, 0x0e, 0x0a, 0x02, 0x67, 0x6b,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x02, 0x67, 0x6b, 0x22, 0x24, 0x0a, 0x0c, 0x43, 0x6f,
	0x6e, 0x73, 0x65, 0x6e, 0x73, 0x75, 0x73, 0x4b, 0x65, 0x79, 0x12, 0x14, 0x0a, 0x05, 0x69, 0x6e,
	0x6e, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x05, 0x69, 0x6e, 0x6e, 0x65, 0x72,
	0x42, 0x94, 0x02, 0x0a, 0x1f, 0x63, 0x6f, 0x6d, 0x2e, 0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72,
	0x61, 0x2e, 0x63, 0x6f, 0x72, 0x65, 0x2e, 0x6b, 0x65, 0x79, 0x73, 0x2e, 0x76, 0x31, 0x61, 0x6c,
	0x70, 0x68, 0x61, 0x31, 0x42, 0x09, 0x4b, 0x65, 0x79, 0x73, 0x50, 0x72, 0x6f, 0x74, 0x6f, 0x50,
	0x01, 0x5a, 0x57, 0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x70, 0x65,
	0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2d, 0x7a, 0x6f, 0x6e, 0x65, 0x2f, 0x70, 0x65, 0x6e, 0x75,
	0x6d, 0x62, 0x72, 0x61, 0x2f, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x2f, 0x67, 0x6f, 0x2f, 0x67, 0x65,
	0x6e, 0x2f, 0x70, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2f, 0x63, 0x6f, 0x72, 0x65, 0x2f,
	0x6b, 0x65, 0x79, 0x73, 0x2f, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x3b, 0x6b, 0x65,
	0x79, 0x73, 0x76, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0xa2, 0x02, 0x03, 0x50, 0x43, 0x4b,
	0xaa, 0x02, 0x1b, 0x50, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x2e, 0x43, 0x6f, 0x72, 0x65,
	0x2e, 0x4b, 0x65, 0x79, 0x73, 0x2e, 0x56, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0xca, 0x02,
	0x1b, 0x50, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x5c, 0x43, 0x6f, 0x72, 0x65, 0x5c, 0x4b,
	0x65, 0x79, 0x73, 0x5c, 0x56, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0xe2, 0x02, 0x27, 0x50,
	0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72, 0x61, 0x5c, 0x43, 0x6f, 0x72, 0x65, 0x5c, 0x4b, 0x65, 0x79,
	0x73, 0x5c, 0x56, 0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x5c, 0x47, 0x50, 0x42, 0x4d, 0x65,
	0x74, 0x61, 0x64, 0x61, 0x74, 0x61, 0xea, 0x02, 0x1e, 0x50, 0x65, 0x6e, 0x75, 0x6d, 0x62, 0x72,
	0x61, 0x3a, 0x3a, 0x43, 0x6f, 0x72, 0x65, 0x3a, 0x3a, 0x4b, 0x65, 0x79, 0x73, 0x3a, 0x3a, 0x56,
	0x31, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x31, 0x62, 0x06, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_penumbra_core_keys_v1alpha1_keys_proto_rawDescOnce sync.Once
	file_penumbra_core_keys_v1alpha1_keys_proto_rawDescData = file_penumbra_core_keys_v1alpha1_keys_proto_rawDesc
)

func file_penumbra_core_keys_v1alpha1_keys_proto_rawDescGZIP() []byte {
	file_penumbra_core_keys_v1alpha1_keys_proto_rawDescOnce.Do(func() {
		file_penumbra_core_keys_v1alpha1_keys_proto_rawDescData = protoimpl.X.CompressGZIP(file_penumbra_core_keys_v1alpha1_keys_proto_rawDescData)
	})
	return file_penumbra_core_keys_v1alpha1_keys_proto_rawDescData
}

var file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes = make([]protoimpl.MessageInfo, 13)
var file_penumbra_core_keys_v1alpha1_keys_proto_goTypes = []interface{}{
	(*Address)(nil),             // 0: penumbra.core.keys.v1alpha1.Address
	(*AddressView)(nil),         // 1: penumbra.core.keys.v1alpha1.AddressView
	(*PayloadKey)(nil),          // 2: penumbra.core.keys.v1alpha1.PayloadKey
	(*SpendKey)(nil),            // 3: penumbra.core.keys.v1alpha1.SpendKey
	(*FullViewingKey)(nil),      // 4: penumbra.core.keys.v1alpha1.FullViewingKey
	(*WalletId)(nil),            // 5: penumbra.core.keys.v1alpha1.WalletId
	(*Diversifier)(nil),         // 6: penumbra.core.keys.v1alpha1.Diversifier
	(*AddressIndex)(nil),        // 7: penumbra.core.keys.v1alpha1.AddressIndex
	(*IdentityKey)(nil),         // 8: penumbra.core.keys.v1alpha1.IdentityKey
	(*GovernanceKey)(nil),       // 9: penumbra.core.keys.v1alpha1.GovernanceKey
	(*ConsensusKey)(nil),        // 10: penumbra.core.keys.v1alpha1.ConsensusKey
	(*AddressView_Decoded)(nil), // 11: penumbra.core.keys.v1alpha1.AddressView.Decoded
	(*AddressView_Opaque)(nil),  // 12: penumbra.core.keys.v1alpha1.AddressView.Opaque
}
var file_penumbra_core_keys_v1alpha1_keys_proto_depIdxs = []int32{
	11, // 0: penumbra.core.keys.v1alpha1.AddressView.decoded:type_name -> penumbra.core.keys.v1alpha1.AddressView.Decoded
	12, // 1: penumbra.core.keys.v1alpha1.AddressView.opaque:type_name -> penumbra.core.keys.v1alpha1.AddressView.Opaque
	0,  // 2: penumbra.core.keys.v1alpha1.AddressView.Decoded.address:type_name -> penumbra.core.keys.v1alpha1.Address
	7,  // 3: penumbra.core.keys.v1alpha1.AddressView.Decoded.index:type_name -> penumbra.core.keys.v1alpha1.AddressIndex
	5,  // 4: penumbra.core.keys.v1alpha1.AddressView.Decoded.wallet_id:type_name -> penumbra.core.keys.v1alpha1.WalletId
	0,  // 5: penumbra.core.keys.v1alpha1.AddressView.Opaque.address:type_name -> penumbra.core.keys.v1alpha1.Address
	6,  // [6:6] is the sub-list for method output_type
	6,  // [6:6] is the sub-list for method input_type
	6,  // [6:6] is the sub-list for extension type_name
	6,  // [6:6] is the sub-list for extension extendee
	0,  // [0:6] is the sub-list for field type_name
}

func init() { file_penumbra_core_keys_v1alpha1_keys_proto_init() }
func file_penumbra_core_keys_v1alpha1_keys_proto_init() {
	if File_penumbra_core_keys_v1alpha1_keys_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*Address); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*AddressView); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[2].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*PayloadKey); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[3].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*SpendKey); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[4].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*FullViewingKey); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[5].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*WalletId); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[6].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*Diversifier); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[7].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*AddressIndex); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[8].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*IdentityKey); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[9].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*GovernanceKey); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[10].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ConsensusKey); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[11].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*AddressView_Decoded); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[12].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*AddressView_Opaque); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
	}
	file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes[1].OneofWrappers = []interface{}{
		(*AddressView_Decoded_)(nil),
		(*AddressView_Opaque_)(nil),
	}
	type x struct{}
	out := protoimpl.TypeBuilder{
		File: protoimpl.DescBuilder{
			GoPackagePath: reflect.TypeOf(x{}).PkgPath(),
			RawDescriptor: file_penumbra_core_keys_v1alpha1_keys_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   13,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_penumbra_core_keys_v1alpha1_keys_proto_goTypes,
		DependencyIndexes: file_penumbra_core_keys_v1alpha1_keys_proto_depIdxs,
		MessageInfos:      file_penumbra_core_keys_v1alpha1_keys_proto_msgTypes,
	}.Build()
	File_penumbra_core_keys_v1alpha1_keys_proto = out.File
	file_penumbra_core_keys_v1alpha1_keys_proto_rawDesc = nil
	file_penumbra_core_keys_v1alpha1_keys_proto_goTypes = nil
	file_penumbra_core_keys_v1alpha1_keys_proto_depIdxs = nil
}
